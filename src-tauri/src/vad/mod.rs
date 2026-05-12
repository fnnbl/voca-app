use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use silero::{
    detect_speech, SampleRate, Session, SpeechOptions, SpeechSegmenter, StreamState,
    BUNDLED_MODEL,
};
use tauri::AppHandle;

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const AUTO_STOP_SILENCE_MS: u64 = 800;
const TRIM_PAD_SECONDS: f32 = 0.1;

/// Apply Silero VAD over a recorded buffer and trim leading + trailing
/// silence. Falls back to returning the original buffer when no speech
/// is detected or the VAD pipeline errors — never drop the recording.
///
/// `samples` is the raw interleaved cpal buffer at `sample_rate` /
/// `channels`. The returned slice is in the same format, so encoding
/// downstream can continue unchanged.
pub fn trim_silence(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<f32> {
    if samples.is_empty() {
        return samples.to_vec();
    }

    let mono_16k = resample_to_mono_16k(samples, sample_rate, channels);
    let mut session = match Session::from_memory(BUNDLED_MODEL) {
        Ok(s) => s,
        Err(e) => {
            log::error!("VAD: failed to construct silero session: {e}");
            return samples.to_vec();
        }
    };
    let opts = SpeechOptions::default().with_sample_rate(SampleRate::Rate16k);

    let segments = match detect_speech(&mut session, &mono_16k, opts) {
        Ok(s) => s,
        Err(e) => {
            log::error!("VAD: detect_speech failed: {e}");
            return samples.to_vec();
        }
    };

    if segments.is_empty() {
        // No speech detected. Pass the buffer through unchanged rather
        // than drop the recording outright — whisper may still produce
        // a sane (or thank-you-flavoured) output, but losing it silently
        // is the worse failure mode.
        return samples.to_vec();
    }

    let first = segments.first().unwrap();
    let last = segments.last().unwrap();
    let start_s = (first.start_seconds() - TRIM_PAD_SECONDS).max(0.0);
    let end_s = last.end_seconds() + TRIM_PAD_SECONDS;

    let samples_per_second = sample_rate as f32 * channels as f32;
    let start_idx = (start_s * samples_per_second) as usize;
    let end_idx = ((end_s * samples_per_second) as usize).min(samples.len());

    if start_idx >= end_idx {
        return samples.to_vec();
    }

    samples[start_idx..end_idx].to_vec()
}

/// Down-mix interleaved multi-channel audio and resample to 16 kHz with
/// linear interpolation. This is the format Silero expects on input.
fn resample_to_mono_16k(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<f32> {
    let mono: Vec<f32> = if channels > 1 {
        let ch = channels as usize;
        samples
            .chunks(ch)
            .map(|c| c.iter().sum::<f32>() / ch as f32)
            .collect()
    } else {
        samples.to_vec()
    };

    if sample_rate == 16_000 {
        return mono;
    }

    let ratio = 16_000.0_f32 / sample_rate as f32;
    let target_len = (mono.len() as f32 * ratio) as usize;
    (0..target_len)
        .map(|i| {
            let src = i as f32 / ratio;
            let floor = src as usize;
            let frac = src - floor as f32;
            let s0 = mono.get(floor).copied().unwrap_or(0.0);
            let s1 = mono.get(floor + 1).copied().unwrap_or(0.0);
            s0 + (s1 - s0) * frac
        })
        .collect()
}

/// Cancellation handle for an active auto-stop watcher thread.
///
/// Storing this in `AudioRecordingState` ties its lifetime to the
/// active recording session — when `audio::stop` clears the slot, the
/// `Drop` impl signals the thread to exit on its next poll.
pub struct AutoStopWatcher {
    stop_flag: Arc<AtomicBool>,
}

impl AutoStopWatcher {
    pub fn cancel(&self) {
        self.stop_flag.store(true, Ordering::Release);
    }
}

impl Drop for AutoStopWatcher {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Spawn the background thread that watches the active recording buffer
/// and triggers a normal stop via `shortcut::stop_recording_external`
/// when Silero reports a finished speech segment — i.e. the user spoke
/// and then paused for at least `AUTO_STOP_SILENCE_MS`.
///
/// If the user never speaks, no segment is emitted and the watcher just
/// idles until the recording is stopped some other way.
pub fn start_auto_stop_watcher(
    app: AppHandle,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
) -> AutoStopWatcher {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_thread = stop_flag.clone();

    thread::spawn(move || {
        let mut session = match Session::from_memory(BUNDLED_MODEL) {
            Ok(s) => s,
            Err(e) => {
                log::error!("VAD watcher: silero session failed: {e}");
                return;
            }
        };
        let opts = SpeechOptions::default()
            .with_sample_rate(SampleRate::Rate16k)
            .with_min_silence_duration(Duration::from_millis(AUTO_STOP_SILENCE_MS));
        let mut stream = StreamState::new(opts.sample_rate());
        let mut segmenter = SpeechSegmenter::new(opts.clone());

        let mut consumed_native: usize = 0;

        loop {
            if stop_flag_thread.load(Ordering::Acquire) {
                return;
            }
            thread::sleep(POLL_INTERVAL);

            let new_samples = {
                let buf = buffer.lock().unwrap();
                if buf.len() <= consumed_native {
                    continue;
                }
                let new = buf[consumed_native..].to_vec();
                consumed_native = buf.len();
                new
            };

            let mono_16k = resample_to_mono_16k(&new_samples, sample_rate, channels);
            let mut segment_ended = false;
            let _ = segmenter.process_samples(&mut session, &mut stream, &mono_16k, |_seg| {
                segment_ended = true;
            });

            if segment_ended {
                crate::shortcut::stop_recording_external(&app);
                return;
            }
        }
    });

    AutoStopWatcher { stop_flag }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_mono_16k_is_passthrough() {
        let input: Vec<f32> = (0..16_000).map(|i| (i as f32 / 16_000.0) * 0.5).collect();
        let out = resample_to_mono_16k(&input, 16_000, 1);
        assert_eq!(out.len(), input.len());
        for (a, b) in input.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn resample_stereo_48k_to_mono_16k() {
        let input: Vec<f32> = vec![0.5; 48_000 * 2]; // 1 s of constant 0.5 stereo
        let out = resample_to_mono_16k(&input, 48_000, 2);
        assert!((out.len() as i64 - 16_000).abs() <= 1);
        for s in &out[100..out.len() - 100] {
            assert!((s - 0.5).abs() < 0.05, "expected ~0.5, got {s}");
        }
    }

    #[test]
    fn resample_stereo_mixes_opposite_phase_to_zero() {
        let mut input: Vec<f32> = Vec::with_capacity(32_000);
        for _ in 0..16_000 {
            input.push(0.5);
            input.push(-0.5);
        }
        let out = resample_to_mono_16k(&input, 16_000, 2);
        for s in &out {
            assert!(s.abs() < 0.01, "stereo mix should be near zero: {s}");
        }
    }

    #[test]
    fn resample_empty_input_stays_empty() {
        let out = resample_to_mono_16k(&[], 44_100, 1);
        assert!(out.is_empty());
    }

    #[test]
    fn trim_silence_empty_input() {
        let out = trim_silence(&[], 16_000, 1);
        assert!(out.is_empty());
    }

    #[test]
    fn trim_silence_pure_silence_returns_original() {
        // 2 s of silence — Silero finds no speech, fallback keeps the buffer
        let input: Vec<f32> = vec![0.0; 32_000];
        let out = trim_silence(&input, 16_000, 1);
        assert_eq!(out.len(), input.len());
    }
}
