// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The queue of text waiting to be spoken. One item is spoken at a time, in the
//! order they arrived, so that two clients never talk over each other.

use serde::Serialize;
use std::collections::VecDeque;

/// One piece of text to be spoken.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub text: String,
    pub client: String,
    pub voice: Option<String>,
    pub speed: Option<f32>,
    pub by_clause: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Queued,
    Speaking,
}

/// An item as clients and the window see it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemView {
    pub id: String,
    pub client: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    pub state: State,
}

impl ItemView {
    fn of(item: &Item, state: State) -> Self {
        Self {
            id: item.id.clone(),
            client: item.client.clone(),
            text: item.text.clone(),
            voice: item.voice.clone(),
            state,
        }
    }
}

/// What withdrawing an item found.
#[derive(Debug, PartialEq)]
pub enum Withdrawn {
    /// It was waiting; it has been removed.
    Queued(Item),
    /// It is being spoken; the player must stop it.
    Speaking(Item),
    NotFound,
}

#[derive(Debug, Default)]
pub struct Queue {
    waiting: VecDeque<Item>,
    speaking: Option<Item>,
}

impl Queue {
    /// Adds an item at the end; returns how many items are ahead of it.
    pub fn push(&mut self, item: Item) -> usize {
        let ahead = self.waiting.len() + usize::from(self.speaking.is_some());
        self.waiting.push_back(item);
        ahead
    }

    /// The next item to speak, which becomes the one speaking; none while one is.
    pub fn take_next(&mut self) -> Option<Item> {
        if self.speaking.is_some() {
            return None;
        }
        self.speaking = self.waiting.pop_front();
        self.speaking.clone()
    }

    /// Ends the item being spoken, if it is `id`.
    pub fn finish(&mut self, id: &str) -> Option<Item> {
        if self.speaking.as_ref().is_some_and(|i| i.id == id) {
            self.speaking.take()
        } else {
            None
        }
    }

    pub fn withdraw(&mut self, id: &str) -> Withdrawn {
        if let Some(pos) = self.waiting.iter().position(|i| i.id == id) {
            return Withdrawn::Queued(self.waiting.remove(pos).expect("position is in range"));
        }
        match &self.speaking {
            Some(i) if i.id == id => Withdrawn::Speaking(i.clone()),
            _ => Withdrawn::NotFound,
        }
    }

    /// Empties the queue: the waiting items, removed, and the one speaking, which the
    /// player must stop and then finish.
    pub fn stop(&mut self) -> (Vec<Item>, Option<Item>) {
        (self.waiting.drain(..).collect(), self.speaking.clone())
    }

    pub fn speaking(&self) -> Option<&Item> {
        self.speaking.as_ref()
    }

    pub fn len(&self) -> usize {
        self.waiting.len() + usize::from(self.speaking.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The one speaking first, then the waiting ones in order.
    pub fn view(&self) -> Vec<ItemView> {
        self.speaking
            .iter()
            .map(|i| ItemView::of(i, State::Speaking))
            .chain(self.waiting.iter().map(|i| ItemView::of(i, State::Queued)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> Item {
        Item {
            id: id.into(),
            text: format!("text {id}"),
            client: "test".into(),
            voice: None,
            speed: None,
            by_clause: None,
        }
    }

    #[test]
    fn items_are_spoken_one_at_a_time_in_order() {
        let mut q = Queue::default();
        assert_eq!(q.push(item("a")), 0);
        assert_eq!(q.push(item("b")), 1);
        assert_eq!(q.take_next().unwrap().id, "a");
        assert_eq!(q.take_next(), None, "nothing more while a is speaking");
        assert_eq!(q.push(item("c")), 2);
        assert_eq!(q.finish("a").unwrap().id, "a");
        assert_eq!(q.take_next().unwrap().id, "b");
    }

    #[test]
    fn finishing_another_item_changes_nothing() {
        let mut q = Queue::default();
        q.push(item("a"));
        q.take_next();
        assert_eq!(q.finish("b"), None);
        assert_eq!(q.speaking().unwrap().id, "a");
    }

    #[test]
    fn a_waiting_item_is_removed_and_a_speaking_one_is_reported() {
        let mut q = Queue::default();
        q.push(item("a"));
        q.push(item("b"));
        q.take_next();
        assert_eq!(q.withdraw("b"), Withdrawn::Queued(item("b")));
        assert_eq!(q.withdraw("a"), Withdrawn::Speaking(item("a")));
        assert_eq!(q.withdraw("z"), Withdrawn::NotFound);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn stop_empties_the_queue_and_names_the_one_speaking() {
        let mut q = Queue::default();
        for id in ["a", "b", "c"] {
            q.push(item(id));
        }
        q.take_next();
        let (removed, speaking) = q.stop();
        assert_eq!(
            removed.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert_eq!(speaking.unwrap().id, "a");
        assert_eq!(q.len(), 1, "a stays until the player finishes it");
    }

    #[test]
    fn the_view_lists_the_speaking_item_first() {
        let mut q = Queue::default();
        q.push(item("a"));
        q.push(item("b"));
        q.take_next();
        let v = q.view();
        assert_eq!((v[0].id.as_str(), v[0].state), ("a", State::Speaking));
        assert_eq!((v[1].id.as_str(), v[1].state), ("b", State::Queued));
    }
}
