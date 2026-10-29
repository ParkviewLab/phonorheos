// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `phonorheos say "…"` sends text to the running app and waits until it has
//! been spoken. Only `--version` exists so far.

use std::process::ExitCode;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--version") => {
            println!("phonorheos {}", phonorheos_core::version());
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: phonorheos --version");
            ExitCode::from(2)
        }
    }
}
