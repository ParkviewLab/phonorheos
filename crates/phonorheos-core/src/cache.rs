// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Speech kept for texts that a client asked to have cached, so that they play at once
//! the next time they are spoken. Nothing is kept unless asked for. The cache lives in
//! memory, is limited to 32 MB, and drops the entry used least recently first.

use std::collections::HashMap;
use std::sync::Arc;

/// The most audio kept, in bytes: about five and a half minutes at 24 kHz.
pub const LIMIT: usize = 32 * 1024 * 1024;

/// What the speech depends on: the text, and the voice, speed and cutting it was made with.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    text: String,
    voice: String,
    speed: u32,
    by_clause: bool,
}

impl Key {
    pub fn new(text: &str, voice: &str, speed: f32, by_clause: bool) -> Self {
        Self {
            text: text.to_string(),
            voice: voice.to_string(),
            speed: speed.to_bits(),
            by_clause,
        }
    }
}

/// The audio of one text, in the chunks the helper made it in.
pub type Chunks = Arc<Vec<Vec<u8>>>;

struct Entry {
    chunks: Chunks,
    size: usize,
    used: u64,
}

pub struct Cache {
    entries: HashMap<Key, Entry>,
    size: usize,
    limit: usize,
    clock: u64,
    generation: u64,
}

impl Default for Cache {
    fn default() -> Self {
        Self::with_limit(LIMIT)
    }
}

impl Cache {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            entries: HashMap::new(),
            size: 0,
            limit,
            clock: 0,
            generation: 0,
        }
    }

    /// The kept audio for `key`, which counts as a use.
    pub fn get(&mut self, key: &Key) -> Option<Chunks> {
        self.clock += 1;
        let e = self.entries.get_mut(key)?;
        e.used = self.clock;
        Some(e.chunks.clone())
    }

    /// Keeps the audio for `key`, dropping the least recently used entries to make room;
    /// audio larger than the whole cache is not kept.
    pub fn put(&mut self, key: Key, chunks: Vec<Vec<u8>>) {
        let size: usize = chunks.iter().map(Vec::len).sum();
        if size > self.limit {
            return;
        }
        if let Some(old) = self.entries.remove(&key) {
            self.size -= old.size;
        }
        while self.size + size > self.limit {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| k.clone())
                .expect("a cache over its limit has entries");
            let e = self
                .entries
                .remove(&oldest)
                .expect("the key was just found");
            self.size -= e.size;
        }
        self.clock += 1;
        self.size += size;
        self.entries.insert(
            key,
            Entry {
                chunks: Arc::new(chunks),
                size,
                used: self.clock,
            },
        );
    }

    /// Empties the cache. Speech begun before the clear is not kept after it: see
    /// `generation`.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.size = 0;
        self.generation += 1;
    }

    /// Counts the clears, so that speech made across one (with the old pronunciation
    /// table, say) can be told apart and not kept.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The bytes of audio kept.
    pub fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> Key {
        Key::new(text, "bm_lewis", 1.0, false)
    }

    #[test]
    fn kept_audio_comes_back_for_the_same_text_voice_and_speed_only() {
        let mut c = Cache::default();
        c.put(key("Thinking..."), vec![vec![1, 2], vec![3]]);
        assert_eq!(
            *c.get(&key("Thinking...")).unwrap(),
            vec![vec![1, 2], vec![3]]
        );
        assert!(
            c.get(&Key::new("Thinking...", "af_heart", 1.0, false))
                .is_none()
        );
        assert!(
            c.get(&Key::new("Thinking...", "bm_lewis", 1.2, false))
                .is_none()
        );
        assert!(
            c.get(&Key::new("Thinking...", "bm_lewis", 1.0, true))
                .is_none()
        );
        assert!(c.get(&key("thinking...")).is_none());
        assert_eq!(c.size(), 3);
    }

    #[test]
    fn the_least_recently_used_entry_goes_first() {
        let mut c = Cache::with_limit(10);
        c.put(key("a"), vec![vec![0; 4]]);
        c.put(key("b"), vec![vec![0; 4]]);
        c.get(&key("a"));
        c.put(key("c"), vec![vec![0; 4]]);
        assert!(c.get(&key("b")).is_none(), "b was used least recently");
        assert!(c.get(&key("a")).is_some() && c.get(&key("c")).is_some());
        assert_eq!(c.size(), 8);
    }

    #[test]
    fn audio_larger_than_the_cache_is_not_kept() {
        let mut c = Cache::with_limit(4);
        c.put(key("a"), vec![vec![0; 2]]);
        c.put(key("long"), vec![vec![0; 5]]);
        assert!(c.get(&key("long")).is_none());
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn putting_a_text_again_replaces_it() {
        let mut c = Cache::with_limit(10);
        c.put(key("a"), vec![vec![0; 6]]);
        c.put(key("a"), vec![vec![0; 8]]);
        assert_eq!((c.len(), c.size()), (1, 8));
        let g = c.generation();
        c.clear();
        assert!(c.is_empty() && c.size() == 0);
        assert_eq!(c.generation(), g + 1);
    }
}
