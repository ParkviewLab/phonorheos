// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Who may reach phonorheos, and how each listener checks it.
//!
//! - The Unix socket is private to the user (mode 0600), so a request on it is let in.
//! - On 127.0.0.1 any local program can connect, web pages in a browser included, so
//!   the `Host` must name the loopback address and port (against DNS rebinding), and a
//!   browser's `Origin` and `Sec-Fetch-Site` must show the request comes from no web page.
//! - On the LAN every request carries a bearer token issued in the window; only the
//!   tokens' SHA-256 hashes are kept.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The listener a request arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Socket,
    /// 127.0.0.1, with the port it listens on.
    Loopback(u16),
    Network,
}

/// A LAN client's token, as stored: its name and the hash of its secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub id: String,
    pub name: String,
    pub hash: String,
    pub created: String,
}

/// A new token's secret, shown once in the window, and the record kept of it.
pub fn issue(name: &str, created: String) -> (String, Token) {
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    let secret = format!("ph_{}", hex::encode(bytes));
    let token = Token {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        hash: hash(&secret),
        created,
    };
    (secret, token)
}

pub fn hash(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// The headers the checks read, lowercased names.
#[derive(Debug, Default, Clone)]
pub struct Headers {
    pub host: Option<String>,
    pub origin: Option<String>,
    pub sec_fetch_site: Option<String>,
    pub authorization: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Let in; for a LAN request, the name of the token it used.
    Allow(Option<String>),
    Deny(u16, &'static str),
}

pub fn decide(source: Source, h: &Headers, tokens: &[Token]) -> Decision {
    match source {
        Source::Socket => Decision::Allow(None),
        Source::Loopback(port) => {
            let hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
            match &h.host {
                Some(host) if hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) => {}
                _ => {
                    return Decision::Deny(
                        403,
                        "the Host must be 127.0.0.1 or localhost with the port",
                    );
                }
            }
            if let Some(origin) = &h.origin {
                let allowed = hosts
                    .iter()
                    .any(|h| origin.eq_ignore_ascii_case(&format!("http://{h}")));
                if !allowed {
                    return Decision::Deny(403, "requests from web pages are refused");
                }
            }
            match h.sec_fetch_site.as_deref() {
                None | Some("none") | Some("same-origin") => Decision::Allow(None),
                Some(_) => Decision::Deny(403, "requests from web pages are refused"),
            }
        }
        Source::Network => {
            let Some(secret) = h
                .authorization
                .as_deref()
                .and_then(|a| a.strip_prefix("Bearer "))
            else {
                return Decision::Deny(401, "a bearer token is required");
            };
            let digest = hash(secret.trim());
            match tokens.iter().find(|t| t.hash == digest) {
                Some(t) => Decision::Allow(Some(t.name.clone())),
                None => Decision::Deny(401, "unknown token"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(h: &str) -> Headers {
        Headers {
            host: Some(h.into()),
            ..Headers::default()
        }
    }

    #[test]
    fn the_socket_is_let_in() {
        assert_eq!(
            decide(Source::Socket, &Headers::default(), &[]),
            Decision::Allow(None)
        );
    }

    #[test]
    fn loopback_needs_the_loopback_host_and_port() {
        let s = Source::Loopback(35861);
        assert_eq!(
            decide(s, &host("127.0.0.1:35861"), &[]),
            Decision::Allow(None)
        );
        assert_eq!(
            decide(s, &host("LOCALHOST:35861"), &[]),
            Decision::Allow(None)
        );
        assert!(matches!(
            decide(s, &host("evil.example:35861"), &[]),
            Decision::Deny(403, _)
        ));
        assert!(matches!(
            decide(s, &host("127.0.0.1:80"), &[]),
            Decision::Deny(403, _)
        ));
        assert!(matches!(
            decide(s, &Headers::default(), &[]),
            Decision::Deny(403, _)
        ));
    }

    #[test]
    fn loopback_refuses_web_pages() {
        let s = Source::Loopback(35861);
        let mut h = host("127.0.0.1:35861");
        h.origin = Some("https://example.com".into());
        assert!(matches!(decide(s, &h, &[]), Decision::Deny(403, _)));
        let mut h = host("127.0.0.1:35861");
        h.sec_fetch_site = Some("cross-site".into());
        assert!(matches!(decide(s, &h, &[]), Decision::Deny(403, _)));
        let mut h = host("127.0.0.1:35861");
        h.origin = Some("http://127.0.0.1:35861".into());
        h.sec_fetch_site = Some("same-origin".into());
        assert_eq!(decide(s, &h, &[]), Decision::Allow(None));
    }

    #[test]
    fn the_lan_needs_a_known_token() {
        let (secret, token) = issue("speed", "now".into());
        assert!(secret.starts_with("ph_") && secret.len() == 51);
        assert_ne!(token.hash, secret, "only the hash is kept");
        let tokens = vec![token];
        let with = |a: &str| Headers {
            authorization: Some(a.into()),
            ..Headers::default()
        };
        assert_eq!(
            decide(Source::Network, &with(&format!("Bearer {secret}")), &tokens),
            Decision::Allow(Some("speed".into()))
        );
        assert_eq!(
            decide(Source::Network, &with("Bearer ph_wrong"), &tokens),
            Decision::Deny(401, "unknown token")
        );
        assert_eq!(
            decide(Source::Network, &Headers::default(), &tokens),
            Decision::Deny(401, "a bearer token is required")
        );
    }
}
