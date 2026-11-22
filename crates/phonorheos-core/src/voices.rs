// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The voices phonorheos speaks with: Kokoro's English voices, which the window
//! offers and `GET /voices` lists. The helper fetches the same list on first run.

use serde::Serialize;

/// The voice for text that names none until the owner chooses another in the window.
pub const FACTORY_DEFAULT: &str = "bm_lewis";

/// Kokoro's English voices; the others need another language's phonemes. Each id is
/// its accent (`a` American, `b` British), its gender (`f`, `m`), `_` and its name.
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

/// One voice as `GET /voices` describes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Voice {
    pub id: &'static str,
    /// The name to show, such as `Emma`.
    pub name: String,
    /// `british` or `american`.
    pub accent: &'static str,
    /// `female` or `male`.
    pub gender: &'static str,
    /// Whether it speaks text that names no voice.
    pub default: bool,
}

/// Describes a voice from its id.
pub fn describe(id: &'static str, default: &str) -> Voice {
    let mut code = id.chars();
    let accent = match code.next() {
        Some('b') => "british",
        _ => "american",
    };
    let gender = match code.next() {
        Some('m') => "male",
        _ => "female",
    };
    let rest = id.split_once('_').map_or(id, |(_, n)| n);
    let mut letters = rest.chars();
    let name = letters
        .next()
        .map(|c| c.to_ascii_uppercase().to_string() + letters.as_str())
        .unwrap_or_default();
    Voice {
        id,
        name,
        accent,
        gender,
        default: id == default,
    }
}

/// Every voice, with the one that speaks text naming none marked.
pub fn all(default: &str) -> Vec<Voice> {
    VOICES.iter().map(|id| describe(id, default)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_voice_is_described_from_its_id() {
        assert_eq!(
            describe("bf_emma", FACTORY_DEFAULT),
            Voice {
                id: "bf_emma",
                name: "Emma".into(),
                accent: "british",
                gender: "female",
                default: false,
            }
        );
        let lewis = describe("bm_lewis", FACTORY_DEFAULT);
        assert_eq!(
            (lewis.accent, lewis.gender, lewis.default),
            ("british", "male", true)
        );
        let adam = describe("am_adam", FACTORY_DEFAULT);
        assert_eq!(
            (adam.name.as_str(), adam.accent, adam.gender),
            ("Adam", "american", "male")
        );
    }

    #[test]
    fn every_id_has_a_known_accent_and_gender() {
        for id in VOICES {
            let (code, name) = id.split_once('_').expect("an id has a _");
            assert!(["af", "am", "bf", "bm"].contains(&code), "{id}");
            assert!(
                !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase()),
                "{id}"
            );
        }
        assert!(VOICES.contains(&FACTORY_DEFAULT));
    }

    #[test]
    fn the_helper_fetches_every_voice_the_app_offers() {
        let engine = include_str!("../../../helper/src/phonorheos_helper/engine.py");
        for id in VOICES {
            let (code, name) = id.split_once('_').unwrap();
            assert!(
                engine.contains(&format!("f\"{code}_{{v}}\""))
                    && engine.contains(&format!("\"{name}\"")),
                "{id} is not in the helper's VOICES"
            );
        }
        assert!(engine.contains(&format!("DEFAULT_VOICE = \"{FACTORY_DEFAULT}\"")));
    }
}
