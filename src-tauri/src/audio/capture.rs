use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                let is_default = default_name.as_ref() == Some(&name);
                devices.push(AudioDeviceInfo { name, is_default });
            }
        }
    }
    devices
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    stream: Option<cpal::Stream>,
    pub channels: u16,
    pub sample_rate: u32,
    device_name: Option<String>,
}

impl AudioRecorder {
    pub fn new(device_name: Option<&str>) -> Result<Self, AppError> {
        let host = cpal::default_host();

        let device = if let Some(name) = device_name {
            if name.is_empty() {
                host.default_input_device()
                    .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?
            } else {
                host.input_devices()
                    .map_err(|e| {
                        AppError::Audio(format!("Failed to enumerate input devices: {}", e))
                    })?
                    .find(|d| d.name().ok().as_deref() == Some(name))
                    .ok_or_else(|| {
                        log::warn!(
                            "Configured input device '{}' not found, falling back to default",
                            name
                        );
                        AppError::Audio(format!("Input device '{}' not found", name))
                    })
                    .or_else(|_| {
                        host.default_input_device().ok_or_else(|| {
                            AppError::Audio("No default input device found".to_string())
                        })
                    })?
            }
        } else {
            host.default_input_device()
                .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?
        };

        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get default input config: {}", e)))?;

        let channels = config.channels();
        let sample_rate = config.sample_rate().0;
        let actual_name = device.name().ok();

        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
            channels,
            sample_rate,
            device_name: actual_name,
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
        let device = if let Some(ref name) = self.device_name {
            host.input_devices()
                .ok()
                .and_then(|mut devs| devs.find(|d| d.name().ok().as_deref() == Some(name)))
                .or_else(|| host.default_input_device())
                .ok_or_else(|| AppError::Audio("No input device found".to_string()))?
        } else {
            host.default_input_device()
                .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?
        };

        let supported_config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get default input config: {}", e)))?;

        // Capture device channel count before consuming the config. The callback
        // downmixes to mono inline so the stored buffer is ~1/device_channels the
        // size of the raw device stream — this is the main memory win for stereo
        // inputs (48 kHz stereo f32 = 384 KB/s → 192 KB/s after downmix).
        let device_channels = supported_config.channels() as usize;
        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();

        let samples_clone = Arc::clone(&self.samples);
        let is_recording_clone = Arc::clone(&self.is_recording);

        let err_fn = |err| {
            eprintln!("Audio stream error: {}", err);
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = samples_clone.lock() {
                                if device_channels <= 1 {
                                    buf.extend_from_slice(data);
                                } else {
                                    buf.reserve(data.len() / device_channels);
                                    let inv_n = 1.0_f32 / device_channels as f32;
                                    for frame in data.chunks_exact(device_channels) {
                                        let mut sum = 0.0_f32;
                                        for &s in frame {
                                            sum += s;
                                        }
                                        buf.push(sum * inv_n);
                                    }
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("Failed to build f32 stream: {}", e)))?,
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = samples_clone.lock() {
                                if device_channels <= 1 {
                                    buf.reserve(data.len());
                                    for &s in data {
                                        buf.push(s as f32 / 32768.0);
                                    }
                                } else {
                                    buf.reserve(data.len() / device_channels);
                                    let inv_n = 1.0_f32 / (device_channels as f32 * 32768.0);
                                    for frame in data.chunks_exact(device_channels) {
                                        let mut sum = 0.0_f32;
                                        for &s in frame {
                                            sum += s as f32;
                                        }
                                        buf.push(sum * inv_n);
                                    }
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
        // Stored buffer is mono regardless of device channel count.
        self.channels = 1;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<f32>, AppError> {
        self.is_recording.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }

        let mut samples = self
            .samples
            .lock()
            .map_err(|e| AppError::Audio(format!("Lock poisoned: {}", e)))?;

        // Move the buffer out instead of cloning to avoid briefly doubling RAM
        // for long recordings (a 30-min mono 48 kHz capture is ~345 MB).
        Ok(std::mem::take(&mut *samples))
    }

    /// Returns a clone of the shared samples buffer for real-time level metering.
    pub fn samples_ref(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.samples)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}
