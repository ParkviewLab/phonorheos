// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use phonorheos_core::access::{Source, issue};
use phonorheos_core::core::{Core, Outcome};
use phonorheos_core::events::EventHub;
use phonorheos_core::server::router;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:35861";

fn core() -> Arc<Core> {
    Arc::new(Core::with_hub(EventHub::starting_at(1)))
}

fn post(path: &str, body: Value) -> Request<Body> {
    Request::post(path)
        .header("host", HOST)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(path: &str) -> Request<Body> {
    Request::get(path)
        .header("host", HOST)
        .body(Body::empty())
        .unwrap()
}

async fn send(core: &Arc<Core>, req: Request<Body>) -> (StatusCode, Value) {
    let res = router(core.clone(), Source::Loopback(35861))
        .oneshot(req)
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn speaking_queues_the_text_and_returns_its_id() {
    let c = core();
    let (code, body) = send(
        &c,
        post("/speak", json!({"text": " Hello. ", "client": "notes"})),
    )
    .await;
    assert_eq!(code, StatusCode::CREATED);
    assert_eq!(
        (body["event"].as_u64(), body["ahead"].as_u64()),
        (Some(1), Some(0))
    );
    let (_, q) = send(&c, get("/queue")).await;
    assert_eq!(q["items"][0]["text"], "Hello.");
    assert_eq!(q["items"][0]["client"], "notes");
    assert_eq!(q["items"][0]["state"], "queued");
}

#[tokio::test]
async fn a_request_may_say_how_its_text_is_cut() {
    for (body, want) in [
        (json!({"text": "a", "by_clause": true}), Some(true)),
        (json!({"text": "a"}), None),
    ] {
        let c = core();
        send(&c, post("/speak", body)).await;
        assert_eq!(c.next().await.by_clause, want);
    }
}

#[tokio::test]
async fn bad_requests_are_refused() {
    let c = core();
    for (body, code) in [
        (json!({"text": "   "}), StatusCode::BAD_REQUEST),
        (json!({"words": "x"}), StatusCode::BAD_REQUEST),
        (json!({"text": "x", "speed": 3.0}), StatusCode::BAD_REQUEST),
        (
            json!({"text": "x", "by_clause": "yes"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"text": "x", "voice": "../etc"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"text": "a".repeat(20_001)}),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        assert_eq!(send(&c, post("/speak", body)).await.0, code);
    }
    assert!(c.queue().is_empty());
}

#[tokio::test]
async fn withdrawing_and_stopping() {
    let c = core();
    let (_, a) = send(&c, post("/speak", json!({"text": "a"}))).await;
    send(&c, post("/speak", json!({"text": "b"}))).await;
    let id = a["id"].as_str().unwrap();
    let del = Request::delete(format!("/speak/{id}"))
        .header("host", HOST)
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&c, del).await.0, StatusCode::NO_CONTENT);
    let again = Request::delete(format!("/speak/{id}"))
        .header("host", HOST)
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&c, again).await.0, StatusCode::NOT_FOUND);
    let (_, stopped) = send(&c, post("/stop", json!({}))).await;
    assert_eq!(stopped["stopped"], 1);
}

#[tokio::test]
async fn a_web_page_and_a_foreign_host_are_refused() {
    let c = core();
    let page = Request::post("/speak")
        .header("host", HOST)
        .header("origin", "https://example.com")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"text":"x"}"#))
        .unwrap();
    assert_eq!(send(&c, page).await.0, StatusCode::FORBIDDEN);
    let rebound = Request::get("/health")
        .header("host", "attacker.example:35861")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&c, rebound).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_lan_names_the_client_by_its_token() {
    let c = core();
    let (secret, token) = issue("speed", "now".into());
    c.set_tokens(vec![token]);
    let lan = router(c.clone(), Source::Network);
    let req = Request::post("/speak")
        .header("authorization", format!("Bearer {secret}"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"text":"x","client":"pretend"}"#))
        .unwrap();
    assert_eq!(
        lan.clone().oneshot(req).await.unwrap().status(),
        StatusCode::CREATED
    );
    assert_eq!(c.queue()[0].client, "speed");
    let anon = Request::get("/status").body(Body::empty()).unwrap();
    assert_eq!(
        lan.oneshot(anon).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn the_stream_replays_an_items_story_from_its_queued_event() {
    let c = core();
    let (_, a) = send(&c, post("/speak", json!({"text": "hi", "client": "cli"}))).await;
    let item = c.next().await;
    c.started(&item);
    c.finished(&item, Outcome::Finished);
    let after = a["event"].as_u64().unwrap();
    let res = router(c.clone(), Source::Loopback(35861))
        .oneshot(get(&format!(
            "/events?item={}&last_id={after}",
            a["id"].as_str().unwrap()
        )))
        .await
        .unwrap();
    assert_eq!(res.headers()["content-type"], "text/event-stream");
    let mut body = res.into_body();
    let mut text = String::new();
    while !text.contains("event: finished") {
        let frame = tokio::time::timeout(Duration::from_secs(2), body.frame())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        text.push_str(&String::from_utf8_lossy(&frame.into_data().unwrap()));
    }
    assert!(text.contains("id: 2\nevent: started\n"), "{text}");
    assert!(text.contains("id: 3\nevent: finished\n"), "{text}");
    assert!(
        !text.contains("event: queued"),
        "the replay starts after the given id"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn end_to_end_over_the_unix_socket_with_curl() {
    let c = core();
    let dir = std::env::temp_dir().join(format!("phonorheos-test-{}", std::process::id()));
    let sock = dir.join("phonorheos.sock");
    tokio::spawn({
        let c = c.clone();
        let sock = sock.clone();
        async move { phonorheos_core::server::serve_socket(c, &sock).await }
    });
    for _ in 0..50 {
        if sock.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&sock).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let out = tokio::process::Command::new("curl")
        .args(["-s", "--unix-socket"])
        .arg(&sock)
        .args([
            "-H",
            "content-type: application/json",
            "-d",
            r#"{"text":"from curl"}"#,
        ])
        .arg("http://localhost/speak")
        .output()
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(body["id"].is_string(), "{body}");
    assert_eq!(
        c.queue()[0].client,
        format!("curl/{}", curl_version().await)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

async fn curl_version() -> String {
    let out = tokio::process::Command::new("curl")
        .arg("--version")
        .output()
        .await
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .to_string()
}

/// With the LAN switched on, one listener serves every interface: a caller on this
/// machine gets the loopback rules (no token, the Host checked), as graptostream does.
#[tokio::test]
async fn on_the_lan_listener_a_local_caller_needs_no_token() {
    let c = core();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn({
        let c = c.clone();
        async move { phonorheos_core::server::serve_tcp(c, listener, true).await }
    });
    let curl = |host: String| async move {
        let out = tokio::process::Command::new("curl")
            .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "-H"])
            .arg(format!("Host: {host}"))
            .arg(format!("http://127.0.0.1:{port}/status"))
            .output()
            .await
            .unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    assert_eq!(curl(format!("127.0.0.1:{port}")).await, "200");
    assert_eq!(curl(format!("evil.example:{port}")).await, "403");
}
