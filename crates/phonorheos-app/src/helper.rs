// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Starts the Python helper, keeps it running, and carries messages both ways.
//!
//! The helper is the app's own Python (`python/` in its resources) running
//! `phonorheos_helper`. When it stops, its state shows the error, and it is started
//! again after a pause that grows to 30 s; the item it was speaking fails.

use base64::Engine as _;
use phonorheos_core::core::{Core, VoiceStatus};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, mpsc};

/// What the helper says about an item.
#[derive(Debug, Clone)]
pub enum Said {
    Audio(String, Vec<u8>),
    Done(String),
    Error(String, String),
    /// The helper stopped; whatever it was speaking will not end otherwise.
    Stopped,
}

/// How to start the helper.
#[derive(Debug, Clone)]
pub struct Launch {
    pub python: PathBuf,
    pub models: PathBuf,
}

/// The app's side of the helper.
#[derive(Clone)]
pub struct Helper {
    requests: mpsc::UnboundedSender<Value>,
    said: broadcast::Sender<Said>,
}

impl Helper {
    pub fn start(launch: Launch, core: Arc<Core>, words: BTreeMap<String, String>) -> Self {
        let (requests, rx) = mpsc::unbounded_channel();
        let said = broadcast::channel(4096).0;
        let helper = Self {
            requests,
            said: said.clone(),
        };
        tauri::async_runtime::spawn(supervise(launch, core, rx, said, words));
        helper
    }

    pub fn said(&self) -> broadcast::Receiver<Said> {
        self.said.subscribe()
    }

    fn send(&self, msg: Value) {
        let _ = self.requests.send(msg);
    }

    pub fn speak(&self, id: &str, text: &str, voice: &str, speed: f32) {
        self.send(json!({"op": "speak", "id": id, "text": text, "voice": voice, "speed": speed}));
    }

    pub fn cancel(&self, id: &str) {
        self.send(json!({"op": "cancel", "id": id}));
    }

    pub fn words(&self, words: &BTreeMap<String, String>) {
        self.send(json!({"op": "words", "words": words}));
    }
}

fn status(state: &str, detail: Option<String>) -> VoiceStatus {
    VoiceStatus {
        state: state.into(),
        detail,
        done: None,
        total: None,
    }
}

async fn supervise(
    launch: Launch,
    core: Arc<Core>,
    mut requests: mpsc::UnboundedReceiver<Value>,
    said: broadcast::Sender<Said>,
    mut words: BTreeMap<String, String>,
) {
    let mut pause = Duration::from_secs(2);
    loop {
        core.set_voice(status("starting", None));
        let started = std::time::Instant::now();
        let why = run_once(&launch, &core, &mut requests, &said, &mut words).await;
        let _ = said.send(Said::Stopped);
        core.set_voice(status("failed", Some(why)));
        if started.elapsed() > Duration::from_secs(60) {
            pause = Duration::from_secs(2);
        }
        tokio::time::sleep(pause).await;
        pause = (pause * 2).min(Duration::from_secs(30));
    }
}

/// Runs the helper until it stops; returns why it stopped.
async fn run_once(
    launch: &Launch,
    core: &Arc<Core>,
    requests: &mut mpsc::UnboundedReceiver<Value>,
    said: &broadcast::Sender<Said>,
    words: &mut BTreeMap<String, String>,
) -> String {
    let mut child = match Command::new(&launch.python)
        .args(["-I", "-B", "-m", "phonorheos_helper"])
        .env("HF_HOME", &launch.models)
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env("TOKENIZERS_PARALLELISM", "false")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return format!(
                "the helper could not start ({}): {e}",
                launch.python.display()
            );
        }
    };
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut lines = BufReader::new(child.stdout.take().expect("piped stdout")).lines();
    let mut errors = BufReader::new(child.stderr.take().expect("piped stderr")).lines();
    let last_error = Arc::new(std::sync::Mutex::new(String::new()));
    let tail = last_error.clone();
    tokio::spawn(async move {
        while let Ok(Some(line)) = errors.next_line().await {
            if !line.trim().is_empty() {
                *tail.lock().expect("stderr lock") = line;
            }
        }
    });
    let mut write =
        async |msg: &Value| stdin.write_all(format!("{msg}\n").as_bytes()).await.is_ok();
    if !write(&json!({"op": "words", "words": words})).await || !write(&json!({"op": "load"})).await
    {
        return "the helper closed its input".into();
    }
    loop {
        tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) => handle(&line, core, said),
                _ => break,
            },
            msg = requests.recv() => match msg {
                Some(msg) => {
                    if msg["op"] == "words"
                        && let Ok(w) = serde_json::from_value(msg["words"].clone())
                    {
                        *words = w;
                    }
                    if !write(&msg).await {
                        break;
                    }
                }
                None => return "the app is closing".into(),
            },
        }
    }
    let code = child
        .wait()
        .await
        .map(|s| s.to_string())
        .unwrap_or_default();
    let last = last_error.lock().expect("stderr lock").clone();
    if last.is_empty() {
        format!("the helper stopped ({code})")
    } else {
        format!("the helper stopped: {last}")
    }
}

fn handle(line: &str, core: &Core, said: &broadcast::Sender<Said>) {
    let Ok(ev) = serde_json::from_str::<Value>(line) else {
        return;
    };
    let id = || ev["id"].as_str().unwrap_or_default().to_string();
    match ev["event"].as_str() {
        Some("progress") => core.set_voice(VoiceStatus {
            state: "downloading".into(),
            detail: ev["file"].as_str().map(str::to_string),
            done: ev["done"].as_u64(),
            total: ev["total"].as_u64(),
        }),
        Some("ready") => core.set_voice(status("ready", None)),
        Some("audio") => {
            if let Some(pcm) = ev["pcm"]
                .as_str()
                .and_then(|p| base64::engine::general_purpose::STANDARD.decode(p).ok())
            {
                let _ = said.send(Said::Audio(id(), pcm));
            }
        }
        Some("done") => {
            let _ = said.send(Said::Done(id()));
        }
        Some("error") => {
            let message = ev["message"].as_str().unwrap_or("unknown").to_string();
            if ev["id"].is_string() {
                let _ = said.send(Said::Error(id(), message));
            } else {
                eprintln!("phonorheos helper: {message}");
            }
        }
        _ => {}
    }
}
