// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The parts of phonorheos that need no window and no voice: the queue, its
//! events, and the HTTP interface that apps reach it by.

pub mod access;
pub mod clock;
pub mod core;
pub mod events;
pub mod paths;
pub mod queue;
pub mod server;

/// The port phonorheos listens on at 127.0.0.1 (and, when switched on, on the LAN);
/// graptostream's is 35860.
pub const PORT: u16 = 35861;

/// The version of phonorheos, from `VERSION.txt` at build time.
pub fn version() -> &'static str {
    include_str!("../../../VERSION.txt").trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_the_version_file_without_its_newline() {
        let v = version();
        assert!(!v.is_empty());
        assert!(!v.contains(char::is_whitespace));
        assert!(v.chars().next().is_some_and(|c| c.is_ascii_digit()));
    }
}
