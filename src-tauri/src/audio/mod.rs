use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{AppHandle, Emitter};

use crate::AudioLevelPayload;

const AUDIO_LEVEL_EMIT_INTERVAL: Duration = Duration::from_millis(33);
const AUDIO_LEVEL_GAIN: f32 = 4.0;

pub struct AudioRecordingState {
    stream: Mutex<Option<cpal::Stream>>,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: Mutex<u32>,
    pub channels: Mutex<u16>,
    /// Holds the VAD auto-stop watcher while a recording is active.
    /// Dropping the value signals the watcher thread to exit.
    pub auto_stop_watcher: Mutex<Option<crate::vad::AutoStopWatcher>>,
}

// cpal::Stream is !Send on Linux (ALSA). All access is serialized via Mutex, so this is safe.
unsafe impl Send for AudioRecordingState {}
unsafe impl Sync for AudioRecordingState {}

impl AudioRecordingState {
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Mutex::new(16000),
            channels: Mutex::new(1),
            auto_stop_watcher: Mutex::new(None),
        }
    }
}

pub struct AudioBuffer(pub Mutex<Option<Vec<u8>>>);

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub fn start(
    audio: &AudioRecordingState,
    device_name: Option<&str>,
    app: &AppHandle,
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = if let Some(name) = device_name {
        host.input_devices()
            .map_err(|e| format!("MICROPHONE_UNAVAILABLE: {e}"))?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or_else(|| format!("MICROPHONE_UNAVAILABLE: device '{name}' not found"))?
    } else {
        host.default_input_device()
            .ok_or_else(|| "MICROPHONE_UNAVAILABLE: no input device found".to_string())?
    };

    let config = device
        .default_input_config()
        .map_err(|e| format!("MICROPHONE_UNAVAILABLE: {e}"))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    *audio.sample_rate.lock().unwrap() = sample_rate;
    *audio.channels.lock().unwrap() = channels;

    let buffer = audio.buffer.clone();
    buffer.lock().unwrap().clear();

    let stream = build_stream(&device, &config, buffer.clone(), app.clone())?;
    stream.play().map_err(|e| format!("RECORDING_FAILED: {e}"))?;

    *audio.stream.lock().unwrap() = Some(stream);

    let auto_stop_enabled = crate::storage::load(app)
        .ok()
        .and_then(|s| s["transcription"]["autoStop"].as_bool())
        .unwrap_or(false);
    if auto_stop_enabled {
        let watcher =
            crate::vad::start_auto_stop_watcher(app.clone(), buffer, sample_rate, channels);
        *audio.auto_stop_watcher.lock().unwrap() = Some(watcher);
    }

    Ok(())
}

pub fn stop(audio: &AudioRecordingState, app: &AppHandle) -> Result<Vec<u8>, String> {
    *audio.stream.lock().unwrap() = None;
    // Drop the watcher first so the worker thread sees the stop_flag on
    // its next poll and won't fire a redundant stop_recording_external.
    *audio.auto_stop_watcher.lock().unwrap() = None;

    let samples = audio.buffer.lock().unwrap().clone();
    let sample_rate = *audio.sample_rate.lock().unwrap();
    let channels = *audio.channels.lock().unwrap();

    let trim_enabled = crate::storage::load(app)
        .ok()
        .and_then(|s| s["transcription"]["trimSilence"].as_bool())
        .unwrap_or(true);
    let trimmed = if trim_enabled {
        crate::vad::trim_silence(&samples, sample_rate, channels)
    } else {
        samples
    };

    encode_wav(&trimmed, sample_rate, channels)
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    app: AppHandle,
) -> Result<cpal::Stream, String> {
    let err_fn = |err| log::error!("audio stream error: {err}");
    let cfg: cpal::StreamConfig = config.clone().into();
    let last_emit = Arc::new(Mutex::new(Instant::now()));

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let emit_state = last_emit.clone();
            let emit_app = app.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _| {
                    buffer.lock().unwrap().extend_from_slice(data);
                    maybe_emit_level(&emit_app, &emit_state, data);
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let emit_state = last_emit.clone();
            let emit_app = app.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[i16], _| {
                    let floats: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    buffer.lock().unwrap().extend_from_slice(&floats);
                    maybe_emit_level(&emit_app, &emit_state, &floats);
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let emit_state = last_emit.clone();
            let emit_app = app.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[u16], _| {
                    let floats: Vec<f32> = data
                        .iter()
                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                        .collect();
                    buffer.lock().unwrap().extend_from_slice(&floats);
                    maybe_emit_level(&emit_app, &emit_state, &floats);
                },
                err_fn,
                None,
            )
        }
        fmt => return Err(format!("RECORDING_FAILED: unsupported sample format {fmt:?}")),
    };

    stream.map_err(|e| format!("RECORDING_FAILED: {e}"))
}

fn maybe_emit_level(app: &AppHandle, last_emit: &Arc<Mutex<Instant>>, samples: &[f32]) {
    let now = Instant::now();
    {
        let mut last = last_emit.lock().unwrap();
        if now.duration_since(*last) < AUDIO_LEVEL_EMIT_INTERVAL {
            return;
        }
        *last = now;
    }

    let level = compute_level(samples);
    let _ = app.emit("audio-level", AudioLevelPayload { level });
}

pub fn compute_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    (rms * AUDIO_LEVEL_GAIN).clamp(0.0, 1.0)
}

fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer =
        hound::WavWriter::new(&mut cursor, spec).map_err(|e| format!("RECORDING_FAILED: {e}"))?;

    for &sample in samples {
        let i16_sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(i16_sample)
            .map_err(|e| format!("RECORDING_FAILED: {e}"))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("RECORDING_FAILED: {e}"))?;

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_wav(bytes: &[u8]) -> (hound::WavSpec, Vec<i16>) {
        let cursor = std::io::Cursor::new(bytes);
        let mut reader = hound::WavReader::new(cursor).unwrap();
        let spec = reader.spec();
        let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        (spec, samples)
    }

    #[test]
    fn encode_wav_produces_valid_wav_header() {
        let samples = vec![0.0f32; 100];
        let bytes = encode_wav(&samples, 16000, 1).unwrap();
        assert!(bytes.starts_with(b"RIFF"));
        assert!(&bytes[8..12] == b"WAVE");
    }

    #[test]
    fn encode_wav_correct_sample_rate_and_channels() {
        let samples = vec![0.5f32; 50];
        let bytes = encode_wav(&samples, 44100, 2).unwrap();
        let (spec, _) = decode_wav(&bytes);
        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.channels, 2);
    }

    #[test]
    fn encode_wav_empty_samples() {
        let bytes = encode_wav(&[], 16000, 1).unwrap();
        let (_, samples) = decode_wav(&bytes);
        assert_eq!(samples.len(), 0);
    }

    #[test]
    fn encode_wav_clamps_out_of_range_samples() {
        let samples = vec![2.0f32, -2.0f32];
        let bytes = encode_wav(&samples, 16000, 1).unwrap();
        let (_, decoded) = decode_wav(&bytes);
        assert_eq!(decoded[0], i16::MAX);
        assert_eq!(decoded[1], i16::MIN + 1); // clamp(-1.0, 1.0) * i16::MAX
    }

    #[test]
    fn encode_wav_preserves_sample_count() {
        let samples: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0) * 2.0 - 1.0).collect();
        let bytes = encode_wav(&samples, 16000, 1).unwrap();
        let (_, decoded) = decode_wav(&bytes);
        assert_eq!(decoded.len(), 1000);
    }

    #[test]
    fn encode_wav_sine_wave_roundtrip_approximate() {
        use std::f32::consts::PI;
        let samples: Vec<f32> = (0..100)
            .map(|i| (2.0 * PI * 440.0 * i as f32 / 16000.0).sin() * 0.5)
            .collect();
        let bytes = encode_wav(&samples, 16000, 1).unwrap();
        let (_, decoded) = decode_wav(&bytes);
        for (orig, dec) in samples.iter().zip(decoded.iter()) {
            let reconstructed = *dec as f32 / i16::MAX as f32;
            assert!((orig - reconstructed).abs() < 0.001, "sample mismatch: {orig} vs {reconstructed}");
        }
    }

    #[test]
    fn compute_level_returns_zero_for_empty() {
        assert_eq!(compute_level(&[]), 0.0);
    }

    #[test]
    fn compute_level_returns_zero_for_silence() {
        let samples = vec![0.0f32; 512];
        assert_eq!(compute_level(&samples), 0.0);
    }

    #[test]
    fn compute_level_scales_rms_by_gain_and_clamps() {
        // Constant 0.5 amplitude → RMS = 0.5 → 0.5 * 4.0 = 2.0 → clamps to 1.0
        let samples = vec![0.5f32; 512];
        assert_eq!(compute_level(&samples), 1.0);
    }

    #[test]
    fn compute_level_matches_rms_for_quiet_signal() {
        // Constant 0.1 amplitude → RMS = 0.1 → 0.1 * 4.0 = 0.4 (within clamp)
        let samples = vec![0.1f32; 512];
        let level = compute_level(&samples);
        assert!((level - 0.4).abs() < 1e-5, "got {level}");
    }

    #[test]
    fn compute_level_symmetric_for_negative_samples() {
        let pos = vec![0.1f32; 512];
        let neg = vec![-0.1f32; 512];
        assert!((compute_level(&pos) - compute_level(&neg)).abs() < 1e-6);
    }
}
