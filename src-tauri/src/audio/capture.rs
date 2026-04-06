use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::errors::AppError;

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    stream: Option<cpal::Stream>,
    pub channels: u16,
    pub sample_rate: u32,
}

impl AudioRecorder {
    pub fn new() -> Result<Self, AppError> {
        let host = cpal::default_host();

        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?;

        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get default input config: {}", e)))?;

        let channels = config.channels();
        let sample_rate = config.sample_rate().0;

        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
            channels,
            sample_rate,
        })
    }

    pub fn start(&mut self) -> Result<(), AppError> {
        {
            let mut samples = self
                .samples
                .lock()
                .map_err(|e| AppError::Audio(format!("Lock poisoned: {}", e)))?;
            samples.clear();
        }

        self.is_recording.store(true, Ordering::SeqCst);

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?;

        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get default input config: {}", e)))?;

        let samples_clone = Arc::clone(&self.samples);
        let is_recording_clone = Arc::clone(&self.is_recording);

        let err_fn = |err| {
            eprintln!("Audio stream error: {}", err);
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = samples_clone.lock() {
                                buf.extend_from_slice(data);
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("Failed to build f32 stream: {}", e)))?,
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = samples_clone.lock() {
                                for &s in data {
                                    buf.push(s as f32 / 32768.0);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("Failed to build i16 stream: {}", e)))?,
            fmt => {
                return Err(AppError::Audio(format!(
                    "Unsupported sample format: {:?}",
                    fmt
                )));
            }
        };

        stream
            .play()
            .map_err(|e| AppError::Audio(format!("Failed to start audio stream: {}", e)))?;

        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<f32>, AppError> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream = None;

        let samples = self
            .samples
            .lock()
            .map_err(|e| AppError::Audio(format!("Lock poisoned: {}", e)))?;

        Ok(samples.clone())
    }

    /// Returns a clone of the shared samples buffer for real-time level metering.
    pub fn samples_ref(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.samples)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}
