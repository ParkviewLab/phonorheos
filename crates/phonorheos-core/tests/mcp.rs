// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use phonorheos_core::access::{Source, issue};
use phonorheos_core::core::Core;
use phonorheos_core::events::EventHub;
use phonorheos_core::server::router;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:35861";

fn core() -> Arc<Core> {
    Arc::new(Core::with_hub(EventHub::starting_at(1)))
}

fn rpc(method: &str, params: Value) -> Request<Body> {
    Request::post("/mcp")
        .header("host", HOST)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-06-18")
        .header("user-agent", "claude-code/test")
        .body(Body::from(
            json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).to_string(),
        ))
        .unwrap()
}

fn call(tool: &str, args: Value) -> Request<Body> {
    rpc("tools/call", json!({"name": tool, "arguments": args}))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The text of a tool result, and whether it is an error.
fn result(body: &Value) -> (String, bool) {
    let r = &body["result"];
    (
        r["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        r["isError"].as_bool().unwrap_or(false),
    )
}

#[tokio::test]
async fn it_initializes_and_lists_say_and_stop() {
    let app = router(core(), Source::Loopback(35861));
    let (code, body) = send(
        &app,
        rpc(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {},
                   "clientInfo": {"name": "test", "version": "0"}}),
        ),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(body["result"]["serverInfo"]["name"], "phonorheos");
    let (_, body) = send(&app, rpc("tools/list", json!({}))).await;
    let mut names: Vec<_> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["say", "stop"]);
}

#[tokio::test]
async fn say_queues_the_text_with_its_voice_and_returns_its_id() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    let (code, body) = send(
        &app,
        call(
            "say",
            json!({"text": " Hello. ", "voice": "bf_emma", "speed": 1.2}),
        ),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let (text, is_error) = result(&body);
    assert!(!is_error, "{text}");
    let reply: Value = serde_json::from_str(&text).unwrap();
    let q = c.queue();
    assert_eq!(reply["id"], q[0].id);
    assert_eq!(reply["ahead"], 0);
    assert_eq!(q[0].text, "Hello.");
    assert_eq!(q[0].voice.as_deref(), Some("bf_emma"));
    assert_eq!(q[0].client, "claude-code/test");
}

#[tokio::test]
async fn say_refuses_what_post_speak_refuses_with_the_same_reason() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    for (args, why) in [
        (json!({"text": "  "}), "the text is empty"),
        (
            json!({"text": "a", "speed": 3.0}),
            "speed must be between 0.5 and 2",
        ),
        (
            json!({"text": "a", "voice": "../x"}),
            "a voice name is letters, digits and _",
        ),
    ] {
        let (_, body) = send(&app, call("say", args)).await;
        assert_eq!(result(&body), (why.to_string(), true));
    }
    assert!(c.queue().is_empty());
}

/// A client of protocol revision 2026-07-28 carries the version, its identity and its
/// capabilities in each request's `_meta`, and the method and tool in headers.
#[tokio::test]
async fn a_2026_07_28_client_is_served() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1.0"},
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    let req = Request::post("/mcp")
        .header("host", HOST)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "tools/call")
        .header("mcp-name", "say")
        .body(Body::from(
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                   "params": {"name": "say", "arguments": {"text": "modern"}, "_meta": meta}})
            .to_string(),
        ))
        .unwrap();
    let (code, body) = send(&app, req).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    assert!(!result(&body).1, "{body}");
    assert_eq!(c.queue()[0].text, "modern");
}

#[tokio::test]
async fn stop_empties_the_queue() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    send(&app, call("say", json!({"text": "one"}))).await;
    send(&app, call("say", json!({"text": "two"}))).await;
    let (_, body) = send(&app, call("stop", json!({}))).await;
    assert_eq!(result(&body).0, r#"{"stopped":2}"#);
    assert!(c.queue().is_empty());
}

#[tokio::test]
async fn only_post_is_served() {
    let app = router(core(), Source::Loopback(35861));
    for method in ["GET", "DELETE"] {
        let req = Request::builder()
            .method(method)
            .uri("/mcp")
            .header("host", HOST)
            .body(Body::empty())
            .unwrap();
        assert_eq!(send(&app, req).await.0, StatusCode::METHOD_NOT_ALLOWED);
    }
}

#[tokio::test]
async fn a_web_page_is_refused() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    let mut req = call("say", json!({"text": "x"}));
    req.headers_mut()
        .insert("origin", "https://example.com".parse().unwrap());
    assert_eq!(send(&app, req).await.0, StatusCode::FORBIDDEN);
    assert!(c.queue().is_empty());
}

#[tokio::test]
async fn on_the_lan_a_token_is_needed_and_names_the_client() {
    let c = core();
    let (secret, token) = issue("speed", "now".into());
    c.set_tokens(vec![token]);
    let lan = router(c.clone(), Source::Network);
    let mut anon = call("say", json!({"text": "x"}));
    anon.headers_mut()
        .insert("host", "pacific.local:35861".parse().unwrap());
    assert_eq!(send(&lan, anon).await.0, StatusCode::UNAUTHORIZED);
    let mut with = call("say", json!({"text": "x"}));
    with.headers_mut()
        .insert("host", "pacific.local:35861".parse().unwrap());
    with.headers_mut()
        .insert("authorization", format!("Bearer {secret}").parse().unwrap());
    let (_, body) = send(&lan, with).await;
    assert!(!result(&body).1);
    assert_eq!(c.queue()[0].client, "speed");
}

/// The endpoint adds next to nothing between a call's arrival and its item reaching
/// the queue: the median call, answered once the item is queued, takes under 5 ms.
#[tokio::test]
async fn a_call_reaches_the_queue_within_5_ms() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    send(&app, call("say", json!({"text": "warm"}))).await;
    let mut times: Vec<Duration> = Vec::new();
    for i in 0..21 {
        let start = Instant::now();
        send(&app, call("say", json!({"text": format!("item {i}")}))).await;
        times.push(start.elapsed());
    }
    times.sort();
    let median = times[times.len() / 2];
    eprintln!("median MCP say call: {median:?}");
    assert!(median < Duration::from_millis(5), "median {median:?}");
    assert_eq!(c.queue().len(), 22);
}

#[tokio::test]
async fn say_carries_the_speak_and_cache_options() {
    let c = core();
    let app = router(c.clone(), Source::Loopback(35861));
    let (_, body) = send(
        &app,
        call(
            "say",
            json!({"text": "One moment please...", "speak": false, "cache": true}),
        ),
    )
    .await;
    assert!(!result(&body).1);
    let item = c.next().await;
    assert!(!item.speak && item.cache);
    let (_, body) = send(&app, call("say", json!({"text": "x", "speak": false}))).await;
    assert!(result(&body).1, "neither speaking nor caching is refused");
}
