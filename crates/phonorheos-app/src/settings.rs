// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The owner's settings, kept as JSON in the data folder.

use phonorheos_core::access::Token;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The voice for items that name none.
    pub voice: String,
    /// The speed for items that name none, 0.5 to 2.
    pub speed: f32,
    /// Whether text is spoken clause by clause as each is made, or made whole first.
    pub by_clause: bool,
    /// Words and how they are said: a respelling, or `/phonemes/`.
    pub words: BTreeMap<String, String>,
    /// Whether the interface is open to the LAN, for clients holding a token.
    pub lan: bool,
    pub tokens: Vec<Token>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            voice: "bm_lewis".into(),
            speed: 1.0,
            by_clause: true,
            // The app says its own name as its owner does (chosen by ear, 2026-10-29).
            words: BTreeMap::from([("phonorheos".into(), "/fQnQ ɹˈiːQs/".into())]),
            lan: false,
            tokens: Vec::new(),
        }
    }
}

impl Settings {
    /// The saved settings, or the defaults where there are none or they cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Saves by writing a new file and renaming it over the old, so that a crash never
    /// leaves half a file; private to the user, since it holds the tokens' hashes.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?,
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_missing_fields_take_their_defaults() {
        let dir = std::env::temp_dir().join(format!("phonorheos-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        assert_eq!(Settings::load(&path), Settings::default());
        let s = Settings {
            voice: "bm_george".into(),
            ..Settings::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).voice, "bm_george");
        std::fs::write(&path, r#"{"speed": 1.5}"#).unwrap();
        let partial = Settings::load(&path);
        assert_eq!((partial.voice.as_str(), partial.speed), ("bm_lewis", 1.5));
        assert!(partial.words.contains_key("phonorheos"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
