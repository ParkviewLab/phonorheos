// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `phonorheos` opens the app; `phonorheos say "…"` (and `stop`, `status`,
//! `--version`) is the command, which talks to the running app and opens no window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod helper;
mod lan;
mod player;
mod settings;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("say" | "stop" | "status" | "--version" | "--help") => phonorheos_cli::run(&args),
        _ => {
            app::run();
            std::process::ExitCode::SUCCESS
        }
    }
}
