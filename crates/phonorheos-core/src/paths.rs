// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Where phonorheos keeps its socket, its settings and its models.
//!
//! - macOS: `~/Library/Application Support/phonorheos/`, the socket `phonorheos.sock` in it.
//! - Linux: `$XDG_DATA_HOME/phonorheos/` (`~/.local/share/phonorheos/`), the socket in
//!   `$XDG_RUNTIME_DIR/phonorheos/` where that is set (it is private to the user and
//!   cleared at logout), else in the data folder.

use std::path::PathBuf;

fn home() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn non_empty(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The folder for settings, models and the socket (on macOS).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = non_empty("PHONORHEOS_DATA") {
        return dir;
    }
    if cfg!(target_os = "macos") {
        home().join("Library/Application Support/phonorheos")
    } else {
        non_empty("XDG_DATA_HOME")
            .unwrap_or_else(|| home().join(".local/share"))
            .join("phonorheos")
    }
}

/// The Unix socket clients reach phonorheos on.
pub fn socket_path() -> PathBuf {
    if let Some(path) = non_empty("PHONORHEOS_SOCKET") {
        return path;
    }
    if cfg!(target_os = "macos") {
        return data_dir().join("phonorheos.sock");
    }
    match non_empty("XDG_RUNTIME_DIR") {
        Some(run) => run.join("phonorheos/phonorheos.sock"),
        None => data_dir().join("phonorheos.sock"),
    }
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

/// Where the helper keeps the models it downloads (its `HF_HOME`).
pub fn models_dir() -> PathBuf {
    data_dir().join("models")
}
