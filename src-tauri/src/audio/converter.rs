use crate::errors::AppError;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

pub fn stereo_to_mono(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks_exact(2)
        .map(|pair| (pair[0] + pair[1]) / 2.0)
        .collect()
}

pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / 32768.0)
        .collect()
}

pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, AppError> {
    if from_rate == 0 {
        return Err(AppError::Audio("from_rate cannot be zero".to_string()));
    }
    if to_rate == 0 {
        return Err(AppError::Audio("to_rate cannot be zero".to_string()));
    }

    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }

    if samples.is_empty() {
        return Ok(vec![]);
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let chunk_size = samples.len();

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(ratio, 2.0, params, chunk_size, 1)
        .map_err(|e| AppError::Audio(format!("Failed to create resampler: {}", e)))?;

    let wave_in = vec![samples.to_vec()];
    let output = resampler
        .process(&wave_in, None)
        .map_err(|e| AppError::Audio(format!("Resampling failed: {}", e)))?;

    Ok(output.into_iter().next().unwrap_or_default())
}

pub fn prepare_for_whisper(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
) -> Result<Vec<f32>, AppError> {
    if samples.is_empty() {
        return Ok(vec![]);
    }

    let mono = if channels > 1 {
        stereo_to_mono(samples)
    } else {
        samples.to_vec()
    };

    resample(&mono, sample_rate, 16000)
}
