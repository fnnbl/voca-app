use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub fn load_context(model_path: &str) -> Result<WhisperContext, String> {
    WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| format!("LOCAL_MODEL_ERROR: failed to load model: {e}"))
}

pub fn transcribe_with_context(ctx: &WhisperContext, wav_bytes: &[u8], language: &str, initial_prompt: Option<&str>) -> Result<String, String> {
    let samples = wav_to_whisper_samples(wav_bytes)?;

    let mut state = ctx
        .create_state()
        .map_err(|e| format!("LOCAL_MODEL_ERROR: {e}"))?;

    let n_threads = std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(4);

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 0 });
    params.set_n_threads(n_threads);
    // Empty/"auto" means auto-detect: pass None so whisper.cpp does the detection
    // itself instead of being forced onto whatever UI-language string leaks in.
    let lang_hint = if language.is_empty() || language.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(language)
    };
    params.set_language(lang_hint);
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    if let Some(prompt) = initial_prompt {
        params.set_initial_prompt(prompt);
    }

    state
        .full(params, &samples)
        .map_err(|e| format!("LOCAL_MODEL_ERROR: inference failed: {e}"))?;

    let n = state.full_n_segments();

    let mut text = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            if let Ok(s) = seg.to_str() {
                text.push_str(s);
            }
        }
    }

    Ok(text.trim().to_owned())
}

pub fn transcribe(wav_bytes: &[u8], language: &str, model_path: &str, initial_prompt: Option<&str>) -> Result<String, String> {
    let ctx = load_context(model_path)?;
    transcribe_with_context(&ctx, wav_bytes, language, initial_prompt)
}

fn wav_to_whisper_samples(wav_bytes: &[u8]) -> Result<Vec<f32>, String> {
    let cursor = std::io::Cursor::new(wav_bytes);
    let mut reader =
        hound::WavReader::new(cursor).map_err(|e| format!("LOCAL_MODEL_ERROR: WAV decode: {e}"))?;
    let spec = reader.spec();

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.unwrap_or(0) as f32 / i16::MAX as f32)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
    };

    let mono: Vec<f32> = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|c| c.iter().sum::<f32>() / spec.channels as f32)
            .collect()
    } else {
        samples
    };

    if spec.sample_rate == 16000 {
        return Ok(mono);
    }

    // Linear interpolation resample to 16kHz
    let ratio = 16000.0_f32 / spec.sample_rate as f32;
    let target_len = (mono.len() as f32 * ratio) as usize;
    let resampled = (0..target_len)
        .map(|i| {
            let src = i as f32 / ratio;
            let floor = src as usize;
            let frac = src - floor as f32;
            let s0 = mono.get(floor).copied().unwrap_or(0.0);
            let s1 = mono.get(floor + 1).copied().unwrap_or(0.0);
            s0 + (s1 - s0) * frac
        })
        .collect();

    Ok(resampled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for &s in samples {
            writer.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn mono_16khz_passthrough_unchanged() {
        let samples: Vec<f32> = (0..160).map(|i| (i as f32 / 160.0) * 0.5).collect();
        let wav = make_wav(&samples, 16000, 1);
        let result = wav_to_whisper_samples(&wav).unwrap();
        assert_eq!(result.len(), samples.len());
    }

    #[test]
    fn stereo_16khz_mixed_to_mono() {
        let left = vec![0.5f32; 100];
        let right = vec![-0.5f32; 100];
        let interleaved: Vec<f32> = left.iter().zip(right.iter()).flat_map(|(&l, &r)| [l, r]).collect();
        let wav = make_wav(&interleaved, 16000, 2);
        let result = wav_to_whisper_samples(&wav).unwrap();
        assert_eq!(result.len(), 100);
        for &s in &result {
            assert!((s).abs() < 0.01, "stereo mix should be near zero: {s}");
        }
    }

    #[test]
    fn resampling_44100_to_16000_produces_correct_length() {
        let samples = vec![0.0f32; 4410]; // 0.1s at 44100Hz
        let wav = make_wav(&samples, 44100, 1);
        let result = wav_to_whisper_samples(&wav).unwrap();
        let expected = (4410.0 * 16000.0 / 44100.0) as usize;
        // allow ±2 for rounding
        assert!((result.len() as i64 - expected as i64).abs() <= 2);
    }

    #[test]
    fn resampling_8000_to_16000_upsamples() {
        let samples = vec![0.5f32; 800]; // 0.1s at 8000Hz
        let wav = make_wav(&samples, 8000, 1);
        let result = wav_to_whisper_samples(&wav).unwrap();
        assert!(result.len() > 800, "upsampled result should be longer");
    }

    #[test]
    fn invalid_wav_bytes_returns_error() {
        let result = wav_to_whisper_samples(b"not a wav file at all");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("LOCAL_MODEL_ERROR"));
    }

    #[test]
    fn empty_wav_body_produces_empty_samples() {
        let wav = make_wav(&[], 16000, 1);
        let result = wav_to_whisper_samples(&wav).unwrap();
        assert_eq!(result.len(), 0);
    }
}
