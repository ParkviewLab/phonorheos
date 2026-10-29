// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What happens to each item, as numbered events. Clients read them as
//! Server-Sent Events; a reconnecting client names the last one it saw and is sent
//! the ones it missed, from a buffer of the most recent.

use crate::clock;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Accepted and waiting.
    Queued,
    /// Its first sound is playing.
    Started,
    /// Spoken to the end.
    Finished,
    /// Withdrawn, or silenced by a stop, before it was spoken to the end.
    Cancelled,
    /// It could not be spoken; `reason` says why.
    Failed,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Queued => "queued",
            Kind::Started => "started",
            Kind::Finished => "finished",
            Kind::Cancelled => "cancelled",
            Kind::Failed => "failed",
        }
    }

    /// Whether the item's story ends here.
    pub fn is_final(self) -> bool {
        matches!(self, Kind::Finished | Kind::Cancelled | Kind::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub id: u64,
    #[serde(rename = "event")]
    pub kind: Kind,
    pub item: String,
    pub client: String,
    pub at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The fields of an event other than its number and time.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub item: String,
    pub client: String,
    pub text: Option<String>,
    pub voice: Option<String>,
    pub reason: Option<String>,
}

const BUFFER: usize = 1000;

struct Inner {
    next: u64,
    recent: VecDeque<Event>,
}

/// Numbers events, keeps the recent ones, and hands each to every subscriber.
pub struct EventHub {
    inner: Mutex<Inner>,
    tx: broadcast::Sender<Event>,
}

impl EventHub {
    /// Numbering starts at the time in milliseconds, so ids keep increasing across
    /// restarts (while the clock does) and a client's Last-Event-ID from an earlier run
    /// never matches a new event by accident.
    pub fn new() -> Self {
        Self::starting_at(clock::now_ms())
    }

    pub fn starting_at(first: u64) -> Self {
        let (tx, _) = broadcast::channel(BUFFER);
        Self {
            inner: Mutex::new(Inner {
                next: first,
                recent: VecDeque::with_capacity(BUFFER),
            }),
            tx,
        }
    }

    pub fn publish(&self, kind: Kind, d: Draft) -> Event {
        let mut inner = self.inner.lock().expect("event hub lock");
        let event = Event {
            id: inner.next,
            kind,
            item: d.item,
            client: d.client,
            at: clock::rfc3339(clock::now_ms()),
            text: d.text,
            voice: d.voice,
            reason: d.reason,
        };
        inner.next += 1;
        if inner.recent.len() == BUFFER {
            inner.recent.pop_front();
        }
        inner.recent.push_back(event.clone());
        // Sent under the lock, so that a subscriber never misses an event between its
        // replay and its first live one. No receivers is not an error.
        let _ = self.tx.send(event.clone());
        event
    }

    /// The events after `after` still in the buffer, and a receiver for those to come.
    pub fn subscribe(&self, after: Option<u64>) -> (Vec<Event>, broadcast::Receiver<Event>) {
        let inner = self.inner.lock().expect("event hub lock");
        let replay = match after {
            Some(last) => inner
                .recent
                .iter()
                .filter(|e| e.id > last)
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        (replay, self.tx.subscribe())
    }

    /// The id the next event will have.
    pub fn next_id(&self) -> u64 {
        self.inner.lock().expect("event hub lock").next
    }
}

impl Default for EventHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(item: &str) -> Draft {
        Draft {
            item: item.into(),
            client: "test".into(),
            ..Draft::default()
        }
    }

    #[test]
    fn events_are_numbered_in_order() {
        let hub = EventHub::starting_at(100);
        assert_eq!(hub.publish(Kind::Queued, draft("a")).id, 100);
        assert_eq!(hub.publish(Kind::Started, draft("a")).id, 101);
        assert_eq!(hub.next_id(), 102);
    }

    #[tokio::test]
    async fn a_subscriber_receives_what_follows() {
        let hub = EventHub::starting_at(1);
        let (replay, mut rx) = hub.subscribe(None);
        assert!(replay.is_empty());
        hub.publish(Kind::Queued, draft("a"));
        assert_eq!(rx.recv().await.unwrap().item, "a");
    }

    #[test]
    fn a_reconnecting_subscriber_is_sent_what_it_missed() {
        let hub = EventHub::starting_at(1);
        for kind in [Kind::Queued, Kind::Started, Kind::Finished] {
            hub.publish(kind, draft("a"));
        }
        let (replay, _) = hub.subscribe(Some(1));
        assert_eq!(
            replay.iter().map(|e| e.kind).collect::<Vec<_>>(),
            [Kind::Started, Kind::Finished]
        );
    }

    #[test]
    fn the_buffer_keeps_only_the_most_recent() {
        let hub = EventHub::starting_at(0);
        for _ in 0..BUFFER + 5 {
            hub.publish(Kind::Queued, draft("a"));
        }
        let (replay, _) = hub.subscribe(Some(0));
        assert_eq!(replay.len(), BUFFER);
        assert_eq!(replay[0].id, 5);
    }

    #[test]
    fn the_json_names_the_event_and_omits_what_is_absent() {
        let hub = EventHub::starting_at(7);
        let e = hub.publish(Kind::Finished, draft("a"));
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["event"], "finished");
        assert_eq!(v["id"], 7);
        assert!(v.get("reason").is_none() && v.get("text").is_none());
    }
}
