// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Speaks the queue, one item at a time: asks the helper for the item's audio, plays
//! it, and reports `started` when its first sound is queued for the device and its
//! end once the device has played the last of it. An item whose speech is cached is
//! played from the cache without asking the helper; an item marked `cache` has its
//! speech kept once made; an item not to be spoken is made and kept, but not played.

use crate::audio::Output;
use crate::helper::{Helper, Said};
use phonorheos_core::cache::Key;
use phonorheos_core::core::{Control, Core, Outcome};
use phonorheos_core::queue::Item;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

/// The voice and speed for items that name none, and how text is cut, kept current from
/// the settings.
#[derive(Debug, Clone)]
pub struct Defaults {
    pub voice: String,
    pub speed: f32,
    pub by_clause: bool,
}

pub async fn run(core: Arc<Core>, helper: Helper, output: Output, defaults: Arc<RwLock<Defaults>>) {
    let mut said = helper.said();
    let mut control = core.control();
    loop {
        let item = core.next().await;
        // The voice may still be loading (the first run downloads it); the item waits.
        while core.voice().state != "ready" {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let d = defaults.read().expect("defaults lock").clone();
        let voice = item.voice.clone().unwrap_or(d.voice);
        let speed = item.speed.unwrap_or(d.speed);
        let by_clause = item.by_clause.unwrap_or(d.by_clause);
        let key = Key::new(&item.text, &voice, speed, by_clause);
        let outcome = if let Some(chunks) = core.cached(&key) {
            if item.speak {
                for pcm in chunks.iter() {
                    output.push(pcm);
                }
                core.started(&item);
                drain(&output, &item, &mut control).await
            } else {
                Outcome::Finished
            }
        } else {
            speak(
                &core,
                &helper,
                &output,
                &item,
                &voice,
                speed,
                by_clause,
                &mut said,
                &mut control,
                key,
            )
            .await
        };
        core.finished(&item, outcome);
    }
}

/// Whether a control message is for this item; one meant for an earlier item, left
/// unread because it came as that item ended, is not.
fn is_for(c: &Control, item: &Item) -> bool {
    match c {
        Control::Stop(id) | Control::Cancel(id) => *id == item.id,
    }
}

#[allow(clippy::too_many_arguments)]
async fn speak(
    core: &Core,
    helper: &Helper,
    output: &Output,
    item: &Item,
    voice: &str,
    speed: f32,
    by_clause: bool,
    said: &mut tokio::sync::broadcast::Receiver<Said>,
    control: &mut tokio::sync::broadcast::Receiver<Control>,
    key: Key,
) -> Outcome {
    // Anything left over from an earlier item is not this one's.
    while said.try_recv().is_ok() {}
    let generation = core.cache_generation();
    helper.speak(&item.id, &item.text, voice, speed, by_clause);
    let mut started = false;
    let mut kept: Vec<Vec<u8>> = Vec::new();
    let mut failure: Option<String> = None;
    loop {
        tokio::select! {
            msg = said.recv() => match msg {
                Ok(Said::Audio(id, pcm)) if id == item.id => {
                    if item.speak {
                        output.push(&pcm);
                        if !started {
                            started = true;
                            core.started(item);
                        }
                    }
                    if item.cache {
                        kept.push(pcm);
                    }
                }
                Ok(Said::Done(id)) if id == item.id => break,
                Ok(Said::Error(id, why)) if id == item.id => failure = Some(why),
                Ok(Said::Stopped) => return Outcome::Failed("the voice stopped".into()),
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return Outcome::Failed("the voice is gone".into()),
            },
            c = control.recv() => if c.as_ref().is_ok_and(|c| is_for(c, item)) {
                helper.cancel(&item.id);
                if item.speak {
                    output.clear();
                }
                return Outcome::Cancelled;
            },
        }
    }
    if let Some(why) = failure {
        if item.speak {
            output.clear();
        }
        return Outcome::Failed(why);
    }
    if item.cache {
        core.keep(key, kept, generation);
    }
    if !item.speak {
        return Outcome::Finished;
    }
    drain(output, item, control).await
}

/// Waits until the device has played every chunk in the buffer, which ends the item.
async fn drain(
    output: &Output,
    item: &Item,
    control: &mut tokio::sync::broadcast::Receiver<Control>,
) -> Outcome {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(20)) => if output.is_drained() || !output.is_open() {
                return Outcome::Finished;
            },
            c = control.recv() => if c.as_ref().is_ok_and(|c| is_for(c, item)) {
                output.clear();
                return Outcome::Cancelled;
            },
        }
    }
}
