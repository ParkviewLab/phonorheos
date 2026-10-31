// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The Model Context Protocol at `POST /mcp`, so that an MCP client (a Claude Code
//! session, say) speaks with a tool call instead of a shell command. Two tools:
//! `say`, which queues text exactly as `POST /speak` does and returns at once, and
//! `stop`. Stateless, with plain JSON replies; the listener's access check (`Host`,
//! `Origin`, LAN tokens) has already run, so rmcp's own `Host` check is off.

use crate::core::Core;
use crate::server::{TokenName, client_name, request};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize, schemars::JsonSchema)]
struct SayParams {
    /// The text to speak.
    text: String,
    /// A Kokoro voice, such as bm_lewis; the app's voice if absent.
    voice: Option<String>,
    /// 0.5 to 2; the app's speed if absent.
    speed: Option<f32>,
}

#[derive(Clone)]
pub struct Phonorheos {
    core: Arc<Core>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Phonorheos {
    #[tool(description = "Speak text aloud. Returns once it is queued.")]
    fn say(
        &self,
        Parameters(p): Parameters<SayParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> CallToolResult {
        let token = parts
            .extensions
            .get::<TokenName>()
            .cloned()
            .unwrap_or(TokenName(None));
        match request(
            &p.text,
            client_name(&token, None, &parts.headers),
            p.voice,
            p.speed,
            None,
        ) {
            Ok(r) => {
                let accepted = self.core.speak(r);
                CallToolResult::success(vec![ContentBlock::text(
                    json!({ "id": accepted.id, "ahead": accepted.ahead }).to_string(),
                )])
            }
            Err((_, why)) => CallToolResult::error(vec![ContentBlock::text(why)]),
        }
    }

    #[tool(description = "Stop speaking and empty the queue.")]
    fn stop(&self) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(
            json!({ "stopped": self.core.stop() }).to_string(),
        )])
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Phonorheos {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("phonorheos", crate::version()))
    }
}

/// The MCP endpoint as a Tower service, mounted at `/mcp` by `server::router`.
pub fn service(core: Arc<Core>) -> StreamableHttpService<Phonorheos, NeverSessionManager> {
    StreamableHttpService::new(
        move || {
            Ok(Phonorheos {
                core: core.clone(),
                tool_router: Phonorheos::tool_router(),
            })
        },
        Arc::new(NeverSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_sse_keep_alive(None)
            .disable_allowed_hosts()
            .with_max_request_body_bytes(256 * 1024),
    )
}
