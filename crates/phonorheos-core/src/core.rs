// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The state the server and the app share: the queue, the events, the tokens, the
//! clients reading the stream, and the voice's state.

use crate::access::Token;
use crate::clock;
use crate::events::{Draft, Event, EventHub, Kind};
use crate::queue::{Item, ItemView, Queue, Withdrawn};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use tokio::sync::{Notify, broadcast, watch};

/// What the player must do to the item it is speaking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Control {
    /// Stop speaking this item.
    Cancel(String),
    /// Stop speaking whatever is being spoken.
    Stop,
}

/// How an item ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Finished,
    Cancelled,
    Failed(String),
}

/// The voice's state, set by the app as its helper starts, downloads and loads.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VoiceStatus {
    /// `starting`, `downloading`, `ready` or `failed`.
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl Default for VoiceStatus {
    fn default() -> Self {
        Self {
            state: "starting".into(),
            detail: None,
            done: None,
            total: None,
        }
    }
}

/// A client reading the event stream.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reader {
    pub name: String,
    pub via: &'static str,
    pub since: String,
}

/// A request to speak, as validated by the server.
#[derive(Debug, Clone)]
pub struct Request {
    pub text: String,
    pub client: String,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

/// What the server returns for an accepted request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Accepted {
    pub id: String,
    /// The `queued` event's id: subscribe with `last_id` set to it to see this item's
    /// `started` and its end, even if they happen before the stream opens.
    pub event: u64,
    /// How many items are ahead of it.
    pub ahead: usize,
}

pub struct Core {
    queue: Mutex<Queue>,
    hub: EventHub,
    wake: Notify,
    control: broadcast::Sender<Control>,
    tokens: RwLock<Vec<Token>>,
    readers: Mutex<BTreeMap<u64, Reader>>,
    next_reader: AtomicU64,
    voice: Mutex<VoiceStatus>,
    changed: watch::Sender<u64>,
}

impl Default for Core {
    fn default() -> Self {
        Self::with_hub(EventHub::new())
    }
}

impl Core {
    pub fn with_hub(hub: EventHub) -> Self {
        Self {
            queue: Mutex::default(),
            hub,
            wake: Notify::new(),
            control: broadcast::channel(64).0,
            tokens: RwLock::default(),
            readers: Mutex::default(),
            next_reader: AtomicU64::new(0),
            voice: Mutex::default(),
            changed: watch::channel(0).0,
        }
    }

    /// Bumped whenever the queue, the readers, the tokens or the voice's state change,
    /// so that the window knows to redraw.
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    fn touch(&self) {
        self.changed.send_modify(|n| *n += 1);
    }

    pub fn events(&self) -> &EventHub {
        &self.hub
    }

    pub fn speak(&self, r: Request) -> Accepted {
        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            text: r.text,
            client: r.client,
            voice: r.voice,
            speed: r.speed,
        };
        let (ahead, event) = {
            let mut q = self.queue.lock().expect("queue lock");
            let ahead = q.push(item.clone());
            let event = self.publish(Kind::Queued, &item, None, true);
            (ahead, event)
        };
        self.wake.notify_one();
        self.touch();
        Accepted {
            id: item.id,
            event: event.id,
            ahead,
        }
    }

    fn publish(&self, kind: Kind, item: &Item, reason: Option<String>, with_text: bool) -> Event {
        self.hub.publish(
            kind,
            Draft {
                item: item.id.clone(),
                client: item.client.clone(),
                text: with_text.then(|| item.text.clone()),
                voice: item.voice.clone(),
                reason,
            },
        )
    }

    /// Withdraws an item; false when there is no such item.
    pub fn withdraw(&self, id: &str) -> bool {
        let found = match self.queue.lock().expect("queue lock").withdraw(id) {
            Withdrawn::Queued(item) => {
                self.publish(Kind::Cancelled, &item, Some("withdrawn".into()), false);
                true
            }
            Withdrawn::Speaking(_) => {
                let _ = self.control.send(Control::Cancel(id.to_string()));
                true
            }
            Withdrawn::NotFound => false,
        };
        if found {
            self.touch();
        }
        found
    }

    /// Silences everything: the waiting items are cancelled, and the player is told to
    /// stop the one speaking. Returns how many items were stopped.
    pub fn stop(&self) -> usize {
        let (removed, speaking) = self.queue.lock().expect("queue lock").stop();
        for item in &removed {
            self.publish(Kind::Cancelled, item, Some("stopped".into()), false);
        }
        if speaking.is_some() {
            let _ = self.control.send(Control::Stop);
        }
        self.touch();
        removed.len() + usize::from(speaking.is_some())
    }

    /// Waits for the next item to speak, which then counts as speaking.
    pub async fn next(&self) -> Item {
        loop {
            let notified = self.wake.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(item) = self.queue.lock().expect("queue lock").take_next() {
                self.touch();
                return item;
            }
            notified.await;
        }
    }

    /// The player calls this when the item's first sound plays.
    pub fn started(&self, item: &Item) {
        self.publish(Kind::Started, item, None, false);
        self.touch();
    }

    /// The player calls this when the item has ended, however it ended.
    pub fn finished(&self, item: &Item, outcome: Outcome) {
        if self
            .queue
            .lock()
            .expect("queue lock")
            .finish(&item.id)
            .is_none()
        {
            return;
        }
        match outcome {
            Outcome::Finished => self.publish(Kind::Finished, item, None, false),
            Outcome::Cancelled => self.publish(
                Kind::Cancelled,
                item,
                Some("stopped while speaking".into()),
                false,
            ),
            Outcome::Failed(why) => self.publish(Kind::Failed, item, Some(why), false),
        };
        self.wake.notify_one();
        self.touch();
    }

    pub fn control(&self) -> broadcast::Receiver<Control> {
        self.control.subscribe()
    }

    pub fn queue(&self) -> Vec<ItemView> {
        self.queue.lock().expect("queue lock").view()
    }

    pub fn tokens(&self) -> Vec<Token> {
        self.tokens.read().expect("tokens lock").clone()
    }

    pub fn set_tokens(&self, tokens: Vec<Token>) {
        *self.tokens.write().expect("tokens lock") = tokens;
        self.touch();
    }

    pub fn voice(&self) -> VoiceStatus {
        self.voice.lock().expect("voice lock").clone()
    }

    pub fn set_voice(&self, status: VoiceStatus) {
        *self.voice.lock().expect("voice lock") = status;
        self.touch();
    }

    /// Registers a stream reader; it is listed until the returned guard is dropped.
    pub fn reader(
        &self,
        core: &std::sync::Arc<Core>,
        name: String,
        via: &'static str,
    ) -> ReaderGuard {
        let id = self.next_reader.fetch_add(1, Ordering::Relaxed);
        self.readers.lock().expect("readers lock").insert(
            id,
            Reader {
                name,
                via,
                since: clock::rfc3339(clock::now_ms()),
            },
        );
        self.touch();
        ReaderGuard {
            core: core.clone(),
            id,
        }
    }

    pub fn readers(&self) -> Vec<Reader> {
        self.readers
            .lock()
            .expect("readers lock")
            .values()
            .cloned()
            .collect()
    }
}

pub struct ReaderGuard {
    core: std::sync::Arc<Core>,
    id: u64,
}

impl Drop for ReaderGuard {
    fn drop(&mut self) {
        self.core
            .readers
            .lock()
            .expect("readers lock")
            .remove(&self.id);
        self.core.touch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn request(text: &str) -> Request {
        Request {
            text: text.into(),
            client: "test".into(),
            voice: None,
            speed: None,
        }
    }

    fn kinds(core: &Core, after: u64) -> Vec<Kind> {
        core.events()
            .subscribe(Some(after))
            .0
            .iter()
            .map(|e| e.kind)
            .collect()
    }

    #[tokio::test]
    async fn an_item_goes_queued_started_finished() {
        let core = Core::with_hub(EventHub::starting_at(10));
        let a = core.speak(request("hello"));
        assert_eq!((a.event, a.ahead), (10, 0));
        let item = core.next().await;
        assert_eq!(item.id, a.id);
        core.started(&item);
        core.finished(&item, Outcome::Finished);
        assert_eq!(
            kinds(&core, 9),
            [Kind::Queued, Kind::Started, Kind::Finished]
        );
        assert!(core.queue().is_empty());
    }

    #[tokio::test]
    async fn next_waits_until_something_is_queued() {
        let core = Arc::new(Core::default());
        let waiting = tokio::spawn({
            let core = core.clone();
            async move { core.next().await.text }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!waiting.is_finished());
        core.speak(request("later"));
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), waiting)
                .await
                .unwrap()
                .unwrap(),
            "later"
        );
    }

    #[tokio::test]
    async fn the_next_item_waits_for_the_one_speaking() {
        let core = Arc::new(Core::default());
        core.speak(request("one"));
        core.speak(request("two"));
        let first = core.next().await;
        let second = tokio::spawn({
            let core = core.clone();
            async move { core.next().await.text }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!second.is_finished(), "two waits until one has finished");
        core.finished(&first, Outcome::Finished);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), second)
                .await
                .unwrap()
                .unwrap(),
            "two"
        );
    }

    #[tokio::test]
    async fn withdrawing_the_item_speaking_tells_the_player() {
        let core = Core::with_hub(EventHub::starting_at(1));
        let mut control = core.control();
        let a = core.speak(request("a"));
        let item = core.next().await;
        assert!(core.withdraw(&a.id));
        assert_eq!(control.recv().await.unwrap(), Control::Cancel(a.id.clone()));
        core.finished(&item, Outcome::Cancelled);
        assert_eq!(kinds(&core, 0), [Kind::Queued, Kind::Cancelled]);
        assert!(!core.withdraw("nothing"));
    }

    #[tokio::test]
    async fn stop_cancels_the_waiting_and_stops_the_speaking() {
        let core = Core::with_hub(EventHub::starting_at(1));
        let mut control = core.control();
        for t in ["a", "b", "c"] {
            core.speak(request(t));
        }
        let item = core.next().await;
        assert_eq!(core.stop(), 3);
        assert_eq!(control.recv().await.unwrap(), Control::Stop);
        core.finished(&item, Outcome::Cancelled);
        let after: Vec<Kind> = kinds(&core, 3);
        assert_eq!(after, [Kind::Cancelled, Kind::Cancelled, Kind::Cancelled]);
    }

    #[test]
    fn readers_are_listed_while_their_guard_lives() {
        let core = Arc::new(Core::default());
        let guard = core.reader(&core, "curl".into(), "socket");
        assert_eq!(core.readers()[0].name, "curl");
        drop(guard);
        assert!(core.readers().is_empty());
    }
}
