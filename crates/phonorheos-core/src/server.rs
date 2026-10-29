// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The HTTP interface, the same on the Unix socket, 127.0.0.1 and the LAN; each
//! listener applies its own access rule (see `access`).
//!
//! - `POST /speak` `{"text", "client"?, "voice"?, "speed"?}` → 201 `{"id", "event", "ahead"}`
//! - `DELETE /speak/{id}` → 204, or 404
//! - `POST /stop` → `{"stopped": n}`
//! - `GET /events?item=&client=&last_id=` (or `Last-Event-ID`) → Server-Sent Events
//! - `GET /queue`, `GET /status`, `GET /health`

use crate::access::{self, Decision, Headers, Source};
use crate::core::{Core, Request};
use crate::events::Event;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request as HttpRequest, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

/// The longest text one request may carry.
pub const MAX_TEXT: usize = 20_000;

#[derive(Clone)]
struct Ctx {
    core: Arc<Core>,
    source: Source,
}

/// The source of a request on a listener that covers every interface: a caller on
/// this machine (a loopback address) gets the loopback rules, any other the LAN's.
fn source_of(ctx: &Ctx, req: &HttpRequest) -> Source {
    match (
        ctx.source,
        req.extensions().get::<ConnectInfo<SocketAddr>>(),
    ) {
        (Source::Network, Some(ConnectInfo(peer))) if peer.ip().is_loopback() => Source::Loopback(
            req.extensions()
                .get::<LocalPort>()
                .map_or(crate::PORT, |p| p.0),
        ),
        (source, _) => source,
    }
}

/// The port a TCP listener serves on.
#[derive(Clone, Copy)]
struct LocalPort(u16);

/// The name a LAN token gave the request, set by the access check.
#[derive(Clone)]
struct TokenName(Option<String>);

pub fn router(core: Arc<Core>, source: Source) -> Router {
    let ctx = Ctx { core, source };
    Router::new()
        .route("/speak", post(speak))
        .route("/speak/{id}", delete(withdraw))
        .route("/stop", post(stop))
        .route("/events", get(events))
        .route("/queue", get(queue))
        .route("/status", get(status))
        .route("/health", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(ctx.clone(), check))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(ctx)
}

fn header(h: &HeaderMap, name: &str) -> Option<String> {
    h.get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

async fn check(State(ctx): State<Ctx>, mut req: HttpRequest, next: Next) -> Response {
    let h = req.headers();
    let headers = Headers {
        host: header(h, "host"),
        origin: header(h, "origin"),
        sec_fetch_site: header(h, "sec-fetch-site"),
        authorization: header(h, "authorization"),
    };
    let source = source_of(&ctx, &req);
    req.extensions_mut().insert(source);
    match access::decide(source, &headers, &ctx.core.tokens()) {
        Decision::Allow(name) => {
            req.extensions_mut().insert(TokenName(name));
            next.run(req).await
        }
        Decision::Deny(code, why) => {
            let code = StatusCode::from_u16(code).unwrap_or(StatusCode::FORBIDDEN);
            (code, Json(json!({ "error": why }))).into_response()
        }
    }
}

fn error(code: StatusCode, why: &str) -> Response {
    (code, Json(json!({ "error": why }))).into_response()
}

#[derive(Deserialize)]
struct SpeakBody {
    text: String,
    client: Option<String>,
    voice: Option<String>,
    speed: Option<f32>,
}

fn client_name(token: &TokenName, given: Option<String>, headers: &HeaderMap) -> String {
    if let Some(name) = &token.0 {
        return name.clone();
    }
    given
        .filter(|c| !c.trim().is_empty())
        .or_else(|| header(headers, "user-agent"))
        .map(|c| c.trim().chars().take(64).collect())
        .unwrap_or_else(|| "unnamed".into())
}

async fn speak(
    State(ctx): State<Ctx>,
    Extension(token): Extension<TokenName>,
    headers: HeaderMap,
    body: Result<Json<SpeakBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return error(StatusCode::BAD_REQUEST, "expected JSON with a text");
    };
    let text = body.text.trim();
    if text.is_empty() {
        return error(StatusCode::BAD_REQUEST, "the text is empty");
    }
    if text.chars().count() > MAX_TEXT {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the text is longer than 20000 characters",
        );
    }
    if let Some(s) = body.speed
        && !(0.5..=2.0).contains(&s)
    {
        return error(StatusCode::BAD_REQUEST, "speed must be between 0.5 and 2");
    }
    let voice = body.voice.filter(|v| !v.trim().is_empty());
    if let Some(v) = &voice
        && !v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return error(
            StatusCode::BAD_REQUEST,
            "a voice name is letters, digits and _",
        );
    }
    let accepted = ctx.core.speak(Request {
        text: text.to_string(),
        client: client_name(&token, body.client, &headers),
        voice,
        speed: body.speed,
    });
    (StatusCode::CREATED, Json(accepted)).into_response()
}

async fn withdraw(State(ctx): State<Ctx>, Path(id): Path<String>) -> Response {
    if ctx.core.withdraw(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        error(StatusCode::NOT_FOUND, "no such item")
    }
}

async fn stop(State(ctx): State<Ctx>) -> Response {
    Json(json!({ "stopped": ctx.core.stop() })).into_response()
}

async fn queue(State(ctx): State<Ctx>) -> Response {
    Json(json!({ "items": ctx.core.queue() })).into_response()
}

async fn status(State(ctx): State<Ctx>) -> Response {
    let items = ctx.core.queue();
    Json(json!({
        "name": "phonorheos",
        "version": crate::version(),
        "voice": ctx.core.voice(),
        "speaking": items.iter().find(|i| i.state == crate::queue::State::Speaking),
        "queued": items.iter().filter(|i| i.state == crate::queue::State::Queued).count(),
        "readers": ctx.core.readers(),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct EventsQuery {
    item: Option<String>,
    client: Option<String>,
    last_id: Option<u64>,
}

fn admits(q: &EventsQuery, e: &Event) -> bool {
    q.item.as_ref().is_none_or(|i| *i == e.item) && q.client.as_ref().is_none_or(|c| *c == e.client)
}

fn frame(e: &Event) -> SseEvent {
    SseEvent::default()
        .id(e.id.to_string())
        .event(e.kind.name())
        .json_data(e)
        .unwrap_or_else(|_| SseEvent::default().comment("unencodable event"))
}

async fn events(
    State(ctx): State<Ctx>,
    Extension(token): Extension<TokenName>,
    Extension(source): Extension<Source>,
    headers: HeaderMap,
    Query(q): Query<EventsQuery>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let after = header(&headers, "last-event-id")
        .and_then(|v| v.trim().parse().ok())
        .or(q.last_id);
    let (replay, rx) = ctx.core.events().subscribe(after);
    let via = match source {
        Source::Socket => "socket",
        Source::Loopback(_) => "loopback",
        Source::Network => "network",
    };
    let name = client_name(
        &token,
        q.client.clone().or_else(|| q.item.clone()),
        &headers,
    );
    let guard = ctx.core.reader(&ctx.core, name, via);
    let q = Arc::new(q);
    let past = {
        let q = q.clone();
        stream::iter(
            replay
                .into_iter()
                .filter(move |e| admits(&q, e))
                .map(|e| Ok(frame(&e))),
        )
    };
    // A reader too far behind to be sent every event is disconnected; it reconnects
    // with Last-Event-ID and is sent what it missed from the buffer.
    let live = stream::unfold((rx, guard), move |(mut rx, guard)| {
        let q = q.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok(e) if admits(&q, &e) => return Some((Ok(frame(&e)), (rx, guard))),
                    Ok(_) => continue,
                    Err(RecvError::Lagged(_) | RecvError::Closed) => return None,
                }
            }
        }
    });
    Sse::new(past.chain(live)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// Serves on a Unix socket, private to the user; a stale socket file is replaced.
#[cfg(unix)]
pub async fn serve_socket(core: Arc<Core>, path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let _ = std::fs::remove_file(path);
    let listener = tokio::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    axum::serve(listener, router(core, Source::Socket)).await
}

/// Serves on a TCP address: 127.0.0.1 (loopback rules) or the LAN (tokens).
pub async fn serve_tcp(
    core: Arc<Core>,
    listener: tokio::net::TcpListener,
    lan: bool,
) -> std::io::Result<()> {
    let port = listener.local_addr()?.port();
    let source = if lan {
        Source::Network
    } else {
        Source::Loopback(port)
    };
    axum::serve(listener, router(core, source)).await
}
