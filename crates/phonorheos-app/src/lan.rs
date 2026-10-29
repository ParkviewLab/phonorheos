// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The TCP listener: on 127.0.0.1 only, or, with the LAN switched on, on every
//! interface (same port), advertised by mDNS as `_phonorheos._tcp`.

use mdns_sd::{ServiceDaemon, ServiceInfo};
use phonorheos_core::core::Core;
use phonorheos_core::server::serve_tcp;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct Tcp {
    core: Arc<Core>,
    task: Option<JoinHandle<()>>,
    mdns: Option<ServiceDaemon>,
    pub lan: bool,
    pub error: Option<String>,
}

impl Tcp {
    pub fn new(core: Arc<Core>) -> Self {
        Self {
            core,
            task: None,
            mdns: None,
            lan: false,
            error: None,
        }
    }

    /// (Re)binds the listener for `lan`. When the LAN's bind fails, it falls back to
    /// 127.0.0.1, so that local programs keep their access, records the LAN as off,
    /// and keeps the error for the window.
    pub async fn set(&mut self, lan: bool) {
        let error = self.bind(lan).await;
        if lan && error.is_some() {
            self.bind(false).await;
        }
        self.error = error;
    }

    async fn bind(&mut self, lan: bool) -> Option<String> {
        if let Some(t) = self.task.take() {
            t.abort();
            let _ = t.await;
        }
        if let Some(d) = self.mdns.take() {
            let _ = d.shutdown();
        }
        let ip = if lan {
            Ipv4Addr::UNSPECIFIED
        } else {
            Ipv4Addr::LOCALHOST
        };
        let addr = SocketAddr::from((ip, phonorheos_core::PORT));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                self.lan = lan;
                let core = self.core.clone();
                self.task = Some(tokio::spawn(async move {
                    let _ = serve_tcp(core, listener, lan).await;
                }));
                if lan && let Err(e) = advertise().map(|d| self.mdns = Some(d)) {
                    return Some(format!("open to the LAN, but not advertised: {e}"));
                }
                None
            }
            Err(e) => {
                self.lan = false;
                Some(format!(
                    "port {} could not be opened: {e}",
                    phonorheos_core::PORT
                ))
            }
        }
    }
}

fn advertise() -> Result<ServiceDaemon, mdns_sd::Error> {
    let daemon = ServiceDaemon::new()?;
    let host = std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "phonorheos".into());
    let info = ServiceInfo::new(
        "_phonorheos._tcp.local.",
        &format!("phonorheos on {host}"),
        &format!("{host}.local."),
        "",
        phonorheos_core::PORT,
        &[("version", phonorheos_core::version())][..],
    )?
    .enable_addr_auto();
    daemon.register(info)?;
    Ok(daemon)
}
