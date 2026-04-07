#[cfg(test)]
mod tests {
    use crate::audio::converter::*;

    // =====================
    // stereo_to_mono tests
    // =====================

    #[test]
    fn test_stereo_to_mono_basic() {
        // [L1, R1, L2, R2] → [(L1+R1)/2, (L2+R2)/2]
        let input = vec![0.0f32, 1.0, 0.5, 0.5];
        let output = stereo_to_mono(&input);
        assert_eq!(output.len(), 2);
        assert!(
            (output[0] - 0.5).abs() < 1e-6,
            "Expected 0.5, got {}",
            output[0]
        );
        assert!(
            (output[1] - 0.5).abs() < 1e-6,
            "Expected 0.5, got {}",
            output[1]
        );
    }

    #[test]
    fn test_stereo_to_mono_empty() {
        let input: Vec<f32> = vec![];
        let output = stereo_to_mono(&input);
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_stereo_to_mono_single_pair() {
        let input = vec![0.2f32, 0.8];
        let output = stereo_to_mono(&input);
        assert_eq!(output.len(), 1);
        assert!(
            (output[0] - 0.5).abs() < 1e-6,
            "Expected 0.5, got {}",
            output[0]
        );
    }

    #[test]
    fn test_stereo_to_mono_identical_channels() {
        let input = vec![0.3f32, 0.3, 0.7, 0.7, -0.5, -0.5];
        let output = stereo_to_mono(&input);
        assert_eq!(output.len(), 3);
        assert!((output[0] - 0.3).abs() < 1e-6);
        assert!((output[1] - 0.7).abs() < 1e-6);
        assert!((output[2] - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_stereo_to_mono_odd_length() {
        // odd number of samples → truncate last incomplete pair
        let input = vec![0.0f32, 1.0, 0.5];
        let output = stereo_to_mono(&input);
        // Should handle gracefully: first pair processed, last lone sample ignored
        assert_eq!(output.len(), 1);
        assert!((output[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_stereo_to_mono_silence() {
        let input = vec![0.0f32; 8];
        let output = stereo_to_mono(&input);
        assert_eq!(output.len(), 4);
        for s in &output {
            assert!((s - 0.0_f32).abs() < 1e-6, "Expected silence, got {}", s);
        }
    }

    // ================
    // i16_to_f32 tests
    // ================

    #[test]
    fn test_i16_to_f32_basic() {
        let input = vec![16384i16, -16384];
        let output = i16_to_f32(&input);
        assert_eq!(output.len(), 2);
        // 16384 / 32768.0 = 0.5
        assert!(
            (output[0] - 0.5).abs() < 1e-4,
            "Expected ~0.5, got {}",
            output[0]
        );
        // -16384 / 32768.0 = -0.5
        assert!(
            (output[1] - (-0.5)).abs() < 1e-4,
            "Expected ~-0.5, got {}",
            output[1]
        );
    }

    #[test]
    fn test_i16_to_f32_max() {
        let input = vec![i16::MAX];
        let output = i16_to_f32(&input);
        assert_eq!(output.len(), 1);
        // i16::MAX (32767) / 32768.0 ≈ 0.99997
        assert!(output[0] > 0.99, "Expected ~1.0, got {}", output[0]);
        assert!(output[0] <= 1.0, "Should not exceed 1.0, got {}", output[0]);
    }

    #[test]
    fn test_i16_to_f32_min() {
        let input = vec![i16::MIN];
        let output = i16_to_f32(&input);
        assert_eq!(output.len(), 1);
        // i16::MIN (-32768) / 32768.0 = -1.0
        assert!(
            output[0] >= -1.0,
            "Should not go below -1.0, got {}",
            output[0]
        );
        assert!(output[0] < -0.99, "Expected ~-1.0, got {}", output[0]);
    }

    #[test]
    fn test_i16_to_f32_zero() {
        let input = vec![0i16];
        let output = i16_to_f32(&input);
        assert_eq!(output.len(), 1);
        assert!(
            (output[0] - 0.0).abs() < 1e-6,
            "Expected 0.0, got {}",
            output[0]
        );
    }

    #[test]
    fn test_i16_to_f32_empty() {
        let input: Vec<i16> = vec![];
        let output = i16_to_f32(&input);
        assert_eq!(output.len(), 0);
    }

    // ================
    // resample tests
    // ================

    #[test]
    fn test_resample_same_rate() {
        let input: Vec<f32> = (0..1600).map(|i| (i as f32) / 1600.0).collect();
        let result = resample(&input, 16000, 16000);
        assert!(result.is_ok(), "Same-rate resample should succeed");
        let output = result.unwrap();
        // Passthrough: output should equal input
        assert_eq!(output.len(), input.len());
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5_f32, "Mismatch: {} vs {}", a, b);
        }
    }

    #[test]
    fn test_resample_48k_to_16k() {
        // 48000 Hz → 16000 Hz: output should be ~1/3 of input length
        let num_samples = 4800usize; // 0.1 seconds at 48kHz
        let input: Vec<f32> = (0..num_samples)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI / 480.0).sin())
            .collect();
        let result = resample(&input, 48000, 16000);
        assert!(result.is_ok(), "48k→16k resample should succeed");
        let output = result.unwrap();
        // Output should be approximately 1600 samples (1/3 of 4800)
        let expected = num_samples * 16000 / 48000;
        let tolerance = (expected as f32 * 0.05) as usize + 10; // 5% tolerance
        assert!(
            output.len() >= expected.saturating_sub(tolerance)
                && output.len() <= expected + tolerance,
            "Expected ~{} samples, got {}",
            expected,
            output.len()
        );
    }

    #[test]
    fn test_resample_44100_to_16k() {
        // 44100 Hz → 16000 Hz
        let num_samples = 4410usize; // 0.1 seconds at 44.1kHz
        let input: Vec<f32> = (0..num_samples)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI / 441.0).sin())
            .collect();
        let result = resample(&input, 44100, 16000);
        assert!(result.is_ok(), "44100→16k resample should succeed");
        let output = result.unwrap();
        // Output should be approximately 1633 samples
        let expected = num_samples * 16000 / 44100;
        let tolerance = (expected as f32 * 0.05) as usize + 10;
        assert!(
            output.len() >= expected.saturating_sub(tolerance)
                && output.len() <= expected + tolerance,
            "Expected ~{} samples, got {}",
            expected,
            output.len()
        );
    }

    #[test]
    fn test_resample_preserves_silence() {
        let input = vec![0.0f32; 4800];
        let result = resample(&input, 48000, 16000);
        assert!(result.is_ok());
        let output = result.unwrap();
        for s in &output {
            assert!(s.abs() < 1e-4_f32, "Expected near-zero output, got {}", s);
        }
    }

    #[test]
    fn test_resample_zero_from_rate_error() {
        let input = vec![0.5f32; 100];
        let result = resample(&input, 0, 16000);
        assert!(result.is_err(), "from_rate=0 should return an error");
    }

    #[test]
    fn test_resample_zero_to_rate_error() {
        let input = vec![0.5f32; 100];
        let result = resample(&input, 48000, 0);
        assert!(result.is_err(), "to_rate=0 should return an error");
    }

    #[test]
    fn test_resample_empty_input() {
        let input: Vec<f32> = vec![];
        let result = resample(&input, 48000, 16000);
        // Empty input should either return empty or an error gracefully
        match result {
            Ok(output) => assert_eq!(output.len(), 0usize),
            Err(_) => {}
        }
    }

    #[test]
    fn test_resample_short_input() {
        // Very few samples — should not crash or panic
        let input = vec![0.1f32, -0.1, 0.2];
        let result = resample(&input, 48000, 16000);
        // Should not panic; result can be ok or err but must not panic
        let _ = result;
    }

    // =======================
    // prepare_for_whisper tests
    // =======================

    #[test]
    fn test_prepare_mono_16k() {
        // Already mono 16kHz → should be passthrough (same data)
        let input: Vec<f32> = (0..1600)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI / 160.0).sin())
            .collect();
        let result = prepare_for_whisper(&input, 1, 16000);
        assert!(result.is_ok(), "mono 16kHz should succeed");
        let output = result.unwrap();
        assert_eq!(output.len(), input.len(), "Passthrough: length must match");
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5_f32, "Mismatch: {} vs {}", a, b);
        }
    }

    #[test]
    fn test_prepare_stereo_48k() {
        // Stereo 48kHz → mono 16kHz
        // Input: 9600 stereo samples (4800 frames at 48kHz = 0.1s)
        let num_stereo_samples = 9600usize;
        let input: Vec<f32> = (0..num_stereo_samples)
            .map(|i| (i as f32 / num_stereo_samples as f32) * 0.5)
            .collect();
        let result = prepare_for_whisper(&input, 2, 48000);
        assert!(result.is_ok(), "stereo 48kHz should succeed");
        let output = result.unwrap();
        // After stereo→mono: 4800 samples; after 48k→16k resample: ~1600 samples
        let expected = num_stereo_samples / 2 * 16000 / 48000;
        let tolerance = (expected as f32 * 0.1) as usize + 10;
        assert!(
            output.len() >= expected.saturating_sub(tolerance)
                && output.len() <= expected + tolerance,
            "Expected ~{} samples (stereo 48k→mono 16k), got {}",
            expected,
            output.len()
        );
    }

    #[test]
    fn test_prepare_mono_48k() {
        // Mono 48kHz → resample only to 16kHz
        let num_samples = 4800usize;
        let input: Vec<f32> = (0..num_samples)
            .map(|i| (i as f32) / num_samples as f32)
            .collect();
        let result = prepare_for_whisper(&input, 1, 48000);
        assert!(result.is_ok(), "mono 48kHz should succeed");
        let output = result.unwrap();
        let expected = num_samples * 16000 / 48000;
        let tolerance = (expected as f32 * 0.05) as usize + 10;
        assert!(
            output.len() >= expected.saturating_sub(tolerance)
                && output.len() <= expected + tolerance,
            "Expected ~{} samples, got {}",
            expected,
            output.len()
        );
    }

    #[test]
    fn test_prepare_stereo_16k() {
        // Stereo 16kHz → mono only, no resample
        let num_stereo_samples = 3200usize; // 1600 frames at 16kHz
        let input: Vec<f32> = (0..num_stereo_samples)
            .map(|i| if i % 2 == 0 { 0.4 } else { 0.6 })
            .collect();
        let result = prepare_for_whisper(&input, 2, 16000);
        assert!(result.is_ok(), "stereo 16kHz should succeed");
        let output = result.unwrap();
        // After stereo→mono: 1600 samples; same rate so no resample
        assert_eq!(
            output.len(),
            num_stereo_samples / 2,
            "Expected {} mono samples, got {}",
            num_stereo_samples / 2,
            output.len()
        );
        // Average of 0.4 and 0.6 = 0.5
        for s in &output {
            assert!((s - 0.5_f32).abs() < 1e-5_f32, "Expected 0.5, got {}", s);
        }
    }

    #[test]
    fn test_prepare_empty() {
        let input: Vec<f32> = vec![];
        let result = prepare_for_whisper(&input, 1, 48000);
        match result {
            Ok(output) => assert_eq!(output.len(), 0usize),
            Err(_) => {}
        }
    }

    #[test]
    #[ignore]
    fn test_audio_recorder_new() {
        use crate::audio::capture::AudioRecorder;
        let result = AudioRecorder::new(None);
        assert!(
            result.is_ok(),
            "AudioRecorder::new() should succeed when an audio device is available"
        );
    }

    #[test]
    #[ignore]
    fn test_audio_recorder_not_recording_initially() {
        use crate::audio::capture::AudioRecorder;
        let recorder = AudioRecorder::new(None).expect("AudioRecorder::new() failed");
        assert!(
            !recorder.is_recording(),
            "Recorder should not be recording initially"
        );
    }
}
