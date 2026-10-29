// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plays the helper's 24 kHz mono audio on the default output device, through cpal.
//!
//! The samples wait in a buffer that the device's callback drains; they are converted
//! to the device's rate (linear interpolation) as they are added, and copied to every
//! channel as they are played. The stream lives on a thread of its own, since cpal's
//! streams may not move between threads, and is opened again after a device error.

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const VOICE_RATE: u32 = 24_000;

struct Shared {
    buffer: Mutex<VecDeque<f32>>,
    /// The device's rate, which the buffer's samples are at.
    rate: Mutex<u32>,
    broken: AtomicBool,
}

/// The player's side of the audio output.
#[derive(Clone)]
pub struct Output {
    shared: Arc<Shared>,
}

impl Output {
    /// Opens the default output device on a thread of its own.
    pub fn start() -> Self {
        let shared = Arc::new(Shared {
            buffer: Mutex::new(VecDeque::new()),
            rate: Mutex::new(VOICE_RATE),
            broken: AtomicBool::new(true),
        });
        let s = shared.clone();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || run(s))
            .expect("the audio thread starts");
        Self { shared }
    }

    /// Adds 16-bit little-endian mono samples at 24 kHz to be played.
    pub fn push(&self, pcm: &[u8]) {
        let samples: Vec<f32> = pcm
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| f32::from(i16::from_le_bytes(*b)) / 32768.0)
            .collect();
        let rate = *self.shared.rate.lock().expect("rate lock");
        let converted = resample(&samples, VOICE_RATE, rate);
        self.shared
            .buffer
            .lock()
            .expect("buffer lock")
            .extend(converted);
    }

    /// Silences at once: everything waiting is dropped.
    pub fn clear(&self) {
        self.shared.buffer.lock().expect("buffer lock").clear();
    }

    /// Whether everything added has been played.
    pub fn is_drained(&self) -> bool {
        self.shared.buffer.lock().expect("buffer lock").is_empty()
    }

    /// Whether a device is open; false while none can be.
    pub fn is_open(&self) -> bool {
        !self.shared.broken.load(Ordering::Relaxed)
    }
}

/// Linear interpolation from one rate to another; enough for speech.
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let n = (samples.len() as u64 * u64::from(to) / u64::from(from)) as usize;
    let step = f64::from(from) / f64::from(to);
    (0..n)
        .map(|i| {
            let pos = i as f64 * step;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = samples[j.min(samples.len() - 1)];
            let b = samples[(j + 1).min(samples.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

fn run(shared: Arc<Shared>) {
    loop {
        match open(&shared) {
            Ok(stream) => {
                shared.broken.store(false, Ordering::Relaxed);
                while !shared.broken.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(250));
                }
                drop(stream);
            }
            Err(e) => {
                eprintln!("phonorheos: no audio output: {e:#}");
                shared.broken.store(true, Ordering::Relaxed);
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn open(shared: &Arc<Shared>) -> Result<cpal::Stream> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| anyhow!("no output device"))?;
    let supported = device
        .default_output_config()
        .context("the output device's format")?;
    *shared.rate.lock().expect("rate lock") = supported.sample_rate();
    let format = supported.sample_format();
    let config = supported.config();
    let stream = match format {
        cpal::SampleFormat::F32 => build::<f32>(&device, config, shared),
        cpal::SampleFormat::I16 => build::<i16>(&device, config, shared),
        cpal::SampleFormat::I32 => build::<i32>(&device, config, shared),
        cpal::SampleFormat::U16 => build::<u16>(&device, config, shared),
        other => Err(anyhow!("unsupported sample format {other:?}")),
    }?;
    stream.play().context("starting the output stream")?;
    Ok(stream)
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    shared: &Arc<Shared>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels.max(1));
    let data = shared.clone();
    let error = shared.clone();
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                let mut buffer = data.buffer.lock().expect("buffer lock");
                for frame in out.chunks_mut(channels) {
                    let s = T::from_sample(buffer.pop_front().unwrap_or(0.0));
                    frame.fill(s);
                }
            },
            move |e| {
                eprintln!("phonorheos: audio output error: {e}");
                error.broken.store(true, Ordering::Relaxed);
            },
            None,
        )
        .context("opening the output stream")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_keeps_the_duration_and_the_shape() {
        let ramp: Vec<f32> = (0..240).map(|i| i as f32 / 240.0).collect();
        let up = resample(&ramp, 24_000, 48_000);
        assert_eq!(up.len(), 480);
        assert!((up[100] - 50.0 / 240.0).abs() < 1e-3);
        assert_eq!(resample(&ramp, 24_000, 24_000), ramp);
        assert_eq!(resample(&ramp, 24_000, 44_100).len(), 441);
    }
}
