// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The app: starts the servers, the helper and the player, and gives the window its
//! state and its controls.

use crate::audio::Output;
use crate::helper::{Helper, Launch};
use crate::lan::Tcp;
use crate::player::{self, Defaults};
use crate::settings::Settings;
use phonorheos_core::access;
use phonorheos_core::clock;
use phonorheos_core::core::{Core, Request};
use phonorheos_core::events::Kind;
use phonorheos_core::paths;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

/// Kokoro's English voices; the others need another language's phonemes.
pub const VOICES: [&str; 28] = [
    "af_alloy",
    "af_aoede",
    "af_bella",
    "af_heart",
    "af_jessica",
    "af_kore",
    "af_nicole",
    "af_nova",
    "af_river",
    "af_sarah",
    "af_sky",
    "am_adam",
    "am_echo",
    "am_eric",
    "am_fenrir",
    "am_liam",
    "am_michael",
    "am_onyx",
    "am_puck",
    "am_santa",
    "bf_alice",
    "bf_emma",
    "bf_isabella",
    "bf_lily",
    "bm_daniel",
    "bm_fable",
    "bm_george",
    "bm_lewis",
];

#[derive(Clone, Serialize)]
struct Spoken {
    at: String,
    client: String,
    text: String,
    outcome: &'static str,
}

struct State {
    core: Arc<Core>,
    helper: Helper,
    settings: Mutex<Settings>,
    defaults: Arc<RwLock<Defaults>>,
    tcp: tokio::sync::Mutex<Tcp>,
    recent: Mutex<VecDeque<Spoken>>,
}

impl State {
    fn save(&self, s: &Settings) -> Result<(), String> {
        s.save(&paths::settings_path())
            .map_err(|e| format!("the settings could not be saved: {e}"))
    }
}

fn python(app: &tauri::App) -> PathBuf {
    if let Some(p) = std::env::var_os("PHONORHEOS_PYTHON") {
        return p.into();
    }
    let res = app.path().resource_dir().unwrap_or_default();
    res.join("python/bin/python3.12")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let settings = Settings::load(&paths::settings_path());
            let core = Arc::new(Core::default());
            core.set_tokens(settings.tokens.clone());
            let helper = Helper::start(
                Launch {
                    python: python(app),
                    models: paths::models_dir(),
                },
                core.clone(),
                settings.words.clone(),
            );
            let defaults = Arc::new(RwLock::new(Defaults {
                voice: settings.voice.clone(),
                speed: settings.speed,
            }));
            let lan = settings.lan;
            let state = Arc::new(State {
                core: core.clone(),
                helper: helper.clone(),
                settings: Mutex::new(settings),
                defaults: defaults.clone(),
                tcp: tokio::sync::Mutex::new(Tcp::new(core.clone())),
                recent: Mutex::new(VecDeque::new()),
            });
            app.manage(state.clone());
            let output = Output::start();
            tauri::async_runtime::spawn(player::run(core.clone(), helper, output, defaults));
            tauri::async_runtime::spawn({
                let core = core.clone();
                async move {
                    let sock = paths::socket_path();
                    if let Err(e) = phonorheos_core::server::serve_socket(core, &sock).await {
                        eprintln!(
                            "phonorheos: the socket {} could not be served: {e}",
                            sock.display()
                        );
                    }
                }
            });
            tauri::async_runtime::spawn({
                let state = state.clone();
                async move { state.tcp.lock().await.set(lan).await }
            });
            tauri::async_runtime::spawn(keep_recent(state.clone()));
            tauri::async_runtime::spawn(tell_the_window(app.handle().clone(), core));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_state,
            set_defaults,
            set_words,
            set_lan,
            issue_token,
            revoke_token,
            stop,
            withdraw,
            try_say,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("phonorheos failed to start");
}

/// The last 50 items spoken (or not), for the window.
async fn keep_recent(state: Arc<State>) {
    let (_, mut rx) = state.core.events().subscribe(None);
    let mut texts: HashMap<String, String> = HashMap::new();
    while let Ok(e) = rx.recv().await {
        if e.kind == Kind::Queued {
            texts.insert(e.item.clone(), e.text.clone().unwrap_or_default());
        } else if e.kind.is_final() {
            let text = texts.remove(&e.item).unwrap_or_default();
            let outcome = match e.kind {
                Kind::Finished => "spoken",
                Kind::Cancelled => "cancelled",
                _ => "failed",
            };
            let mut recent = state.recent.lock().expect("recent lock");
            recent.push_front(Spoken {
                at: e.at.clone(),
                client: e.client.clone(),
                text,
                outcome,
            });
            recent.truncate(50);
        }
    }
}

/// Tells the window to redraw when anything changes, at most ten times a second.
async fn tell_the_window(app: tauri::AppHandle, core: Arc<Core>) {
    let mut changes = core.changes();
    while changes.changed().await.is_ok() {
        tokio::time::sleep(Duration::from_millis(100)).await;
        changes.borrow_and_update();
        let _ = app.emit("changed", ());
    }
}

type S<'a> = tauri::State<'a, Arc<State>>;

#[tauri::command]
async fn app_state(app: tauri::AppHandle, state: S<'_>) -> Result<Value, String> {
    let (lan_error, lan) = {
        let tcp = state.tcp.lock().await;
        (tcp.error.clone(), tcp.lan)
    };
    let s = state.settings.lock().expect("settings lock").clone();
    let tokens: Vec<Value> = s
        .tokens
        .iter()
        .map(|t| json!({"id": t.id, "name": t.name, "created": t.created}))
        .collect();
    Ok(json!({
        "version": phonorheos_core::version(),
        "socket": paths::socket_path(),
        "port": phonorheos_core::PORT,
        "voice": state.core.voice(),
        "queue": state.core.queue(),
        "readers": state.core.readers(),
        "recent": *state.recent.lock().expect("recent lock"),
        "voices": VOICES,
        "settings": {"voice": s.voice, "speed": s.speed, "words": s.words, "lan": lan, "tokens": tokens},
        "lan_error": lan_error,
        "autostart": app.autolaunch().is_enabled().unwrap_or(false),
    }))
}

#[tauri::command]
fn set_defaults(state: S<'_>, voice: String, speed: f32) -> Result<(), String> {
    if !VOICES.contains(&voice.as_str()) {
        return Err(format!("no such voice: {voice}"));
    }
    let speed = speed.clamp(0.5, 2.0);
    let mut s = state.settings.lock().expect("settings lock");
    s.voice = voice.clone();
    s.speed = speed;
    state.save(&s)?;
    *state.defaults.write().expect("defaults lock") = Defaults { voice, speed };
    Ok(())
}

#[tauri::command]
fn set_words(state: S<'_>, words: BTreeMap<String, String>) -> Result<(), String> {
    let words: BTreeMap<String, String> = words
        .into_iter()
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .collect();
    let mut s = state.settings.lock().expect("settings lock");
    s.words = words;
    state.save(&s)?;
    state.helper.words(&s.words);
    Ok(())
}

#[tauri::command]
async fn set_lan(state: S<'_>, on: bool) -> Result<(), String> {
    {
        let mut s = state.settings.lock().expect("settings lock");
        s.lan = on;
        state.save(&s)?;
    }
    let mut tcp = state.tcp.lock().await;
    tcp.set(on).await;
    if tcp.lan != on {
        // The LAN could not be opened: keep the setting as it really is.
        let mut s = state.settings.lock().expect("settings lock");
        s.lan = tcp.lan;
        state.save(&s)?;
    }
    match &tcp.error {
        Some(e) => Err(e.clone()),
        None => Ok(()),
    }
}

/// Issues a LAN token; its secret is shown once and never kept.
#[tauri::command]
fn issue_token(state: S<'_>, name: String) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a token needs a name, such as the machine it is for".into());
    }
    let (secret, token) = access::issue(name, clock::rfc3339(clock::now_ms()));
    let mut s = state.settings.lock().expect("settings lock");
    s.tokens.push(token);
    state.save(&s)?;
    state.core.set_tokens(s.tokens.clone());
    Ok(secret)
}

#[tauri::command]
fn revoke_token(state: S<'_>, id: String) -> Result<(), String> {
    let mut s = state.settings.lock().expect("settings lock");
    s.tokens.retain(|t| t.id != id);
    state.save(&s)?;
    state.core.set_tokens(s.tokens.clone());
    Ok(())
}

#[tauri::command]
fn stop(state: S<'_>) -> usize {
    state.core.stop()
}

#[tauri::command]
fn withdraw(state: S<'_>, id: String) -> bool {
    state.core.withdraw(&id)
}

/// Speaks a line from the window, such as a word list entry to check.
#[tauri::command]
fn try_say(state: S<'_>, text: String, voice: Option<String>) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("nothing to say".into());
    }
    let accepted = state.core.speak(Request {
        text: text.to_string(),
        client: "phonorheos window".into(),
        voice: voice.filter(|v| VOICES.contains(&v.as_str())),
        speed: None,
    });
    Ok(accepted.id)
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, on: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let r = if on {
        launcher.enable()
    } else {
        launcher.disable()
    };
    r.map_err(|e| e.to_string())
}
