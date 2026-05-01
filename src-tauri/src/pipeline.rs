use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use serde::Serialize;
use crate::audio::capture::AudioRecorder;
use crate::audio::converter;
use crate::stt::SharedTranscriber;
use crate::llm::client::{HistoryEntry, LlmClient};
use crate::input::paste;
use crate::config;
use crate::errors::AppError;
use crate::history;

/// Shared generation counter for overlay hide scheduling.
/// Every time the shortcut is pressed (overlay shown), the counter increments.
/// Delayed hide tasks check the counter before hiding — if it changed,
/// a newer press occurred and the hide is skipped.
pub type OverlayGeneration = Arc<AtomicU64>;

pub fn new_overlay_generation() -> OverlayGeneration {
    Arc::new(AtomicU64::new(0))
}

/// Conditionally hide the overlay window only if the generation hasn't changed.
/// Returns true if the window was hidden, false if skipped due to generation mismatch.
pub fn hide_overlay_if_current(app: &AppHandle, gen: &OverlayGeneration, expected: u64) -> bool {
    if gen.load(Ordering::SeqCst) != expected {
        return false;
    }
    crate::commands::window::hide_overlay_async(app);
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Idle,
    Recording,
    Transcribing,
    Optimizing,
    Pasting,
    Done { transcribed_text: String, text: String },
    Error { message: String },
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self { flag: Arc::new(AtomicBool::new(false)) }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineEvent {
    pub state: PipelineState,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineWarning {
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelEvent {
    pub level: f32,
}

pub struct Pipeline {
    recorder: Option<AudioRecorder>,
    cancel_token: CancellationToken,
    active: Arc<AtomicBool>,
    level_task: Option<tauri::async_runtime::JoinHandle<()>>,
    source_app: Option<String>,
}

unsafe impl Send for Pipeline {}
unsafe impl Sync for Pipeline {}

pub type PipelineActive = Arc<AtomicBool>;

pub fn new_pipeline_active() -> PipelineActive {
    Arc::new(AtomicBool::new(false))
}

pub struct RecordedAudio {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub source_app: Option<String>,
}

impl Pipeline {
    pub fn new(active: PipelineActive) -> Self {
        Self { recorder: None, cancel_token: CancellationToken::new(), active, level_task: None, source_app: None }
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn set_source_app(&mut self, app: Option<String>) {
        self.source_app = app;
    }

    pub fn cancel(&mut self, app: &AppHandle) {
        self.active.store(false, Ordering::SeqCst);
        self.cancel_token.cancel();

        if let Some(handle) = self.level_task.take() {
            handle.abort();
        }

        if let Some(mut recorder) = self.recorder.take() {
            let _ = recorder.stop();
        }

        let _ = app.emit(
            "pipeline-state",
            PipelineEvent {
                state: PipelineState::Cancelled,
            },
        );
    }

    pub fn is_recording(&self) -> bool {
        self.recorder
            .as_ref()
            .map(|r| r.is_recording())
            .unwrap_or(false)
    }

    pub fn start_recording(&mut self, app: &AppHandle) -> Result<(), AppError> {
        if self.recorder.is_some() {
            return Err(AppError::Audio("Already recording".to_string()));
        }

        self.cancel_token.reset();

        let transcriber = app.state::<SharedTranscriber>();
        let is_loaded = transcriber
            .lock()
            .map(|t| t.is_loaded())
            .unwrap_or(false);

        if !is_loaded {
            return Err(AppError::Whisper(
                "STT model not loaded. Please download and select a model in Settings.".to_string(),
            ));
        }

        let device_name = config::load().ok().map(|c| c.input_device).filter(|s| !s.is_empty());
        let mut recorder = AudioRecorder::new(device_name.as_deref())?;
        recorder.start()?;
        let samples_ref = recorder.samples_ref();
        self.recorder = Some(recorder);
        self.active.store(true, Ordering::SeqCst);

        let app_clone = app.clone();
        let cancel_clone = self.cancel_token.clone();
        self.level_task = Some(tauri::async_runtime::spawn(async move {
            use tokio::time::{interval, Duration};
            let mut ticker = interval(Duration::from_millis(50));
            let mut prev_len: usize = 0;
            loop {
                ticker.tick().await;
                if cancel_clone.is_cancelled() {
                    break;
                }
                let level = {
                    let guard = match samples_ref.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    let len = guard.len();
                    let window_size = 2400; // ~50ms at 48kHz
                    let start = if len > window_size { len - window_size } else { 0 };
                    let window = &guard[start..];
                    if window.is_empty() || len == prev_len {
                        prev_len = len;
                        0.0_f32
                    } else {
                        prev_len = len;
                        let sum_sq: f32 = window.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / window.len() as f32).sqrt();
                        // Scale RMS to 0..1 range (typical speech RMS ~0.02-0.15)
                        (rms * 14.0).min(1.0)
                    }
                };
                let _ = app_clone.emit("audio-level", AudioLevelEvent { level });
            }
        }));

        let _ = app.emit(
            "pipeline-state",
            PipelineEvent {
                state: PipelineState::Recording,
            },
        );
        Ok(())
    }

    pub fn stop_recording_sync(&mut self) -> Result<RecordedAudio, AppError> {
        self.active.store(false, Ordering::SeqCst);

        if let Some(handle) = self.level_task.take() {
            handle.abort();
        }

        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| AppError::Audio("Not recording".to_string()))?;

        let channels = recorder.channels;
        let sample_rate = recorder.sample_rate;
        let samples = recorder.stop()?;
        let source_app = self.source_app.take();

        Ok(RecordedAudio { samples, channels, sample_rate, source_app })
    }
}

pub async fn process_audio(recorded: RecordedAudio, app: AppHandle, cancel: CancellationToken) -> Result<String, AppError> {
    if cancel.is_cancelled() {
        return Err(AppError::Audio("Cancelled".to_string()));
    }

    let _ = app.emit(
        "pipeline-state",
        PipelineEvent {
            state: PipelineState::Transcribing,
        },
    );

    // Destructure so the raw sample Vec can be dropped the moment it's no
    // longer needed. Otherwise `recorded` lives until the end of this async
    // fn and pins the raw buffer in memory while STT runs on the resampled copy.
    let RecordedAudio {
        samples,
        channels,
        sample_rate,
        source_app,
    } = recorded;
    let audio = converter::prepare_for_whisper(&samples, channels, sample_rate)?;
    drop(samples);

    if cancel.is_cancelled() {
        return Err(AppError::Audio("Cancelled".to_string()));
    }

    let config = config::load()?;
    let language = config.language.clone();
    let transcriber = app.state::<SharedTranscriber>().inner().clone();
    let cancel_for_blocking = cancel.clone();
    let text = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::task::spawn_blocking(move || {
            if cancel_for_blocking.is_cancelled() {
                return Err(AppError::Audio("Cancelled".to_string()));
            }
            let guard = transcriber
                .lock()
                .map_err(|e| AppError::Whisper(format!("Transcriber mutex poisoned: {}", e)))?;
            guard.transcribe(&audio, &language)
        }),
    )
        .await
        .map_err(|_| AppError::Whisper("STT transcription timed out (120s)".to_string()))?
        .map_err(|e| AppError::Whisper(e.to_string()))??;

    if cancel.is_cancelled() {
        return Err(AppError::Audio("Cancelled".to_string()));
    }

    if text.trim().is_empty() {
        let _ = app.emit(
            "pipeline-state",
            PipelineEvent {
                state: PipelineState::Done {
                    transcribed_text: String::new(),
                    text: String::new(),
                },
            },
        );
        let gen = app.state::<OverlayGeneration>().inner().clone();
        let snap = gen.load(Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        hide_overlay_if_current(&app, &gen, snap);
        return Ok(String::new());
    }

    if cancel.is_cancelled() {
        return Err(AppError::Audio("Cancelled".to_string()));
    }

    let final_text = if !config.api_key.is_empty() && !config.model.is_empty() {
        let _ = app.emit(
            "pipeline-state",
            PipelineEvent {
                state: PipelineState::Optimizing,
            },
        );
        let client = LlmClient::new(config.api_key, config.api_base_url, Some(config.model.clone()))?;

        let history = history::load_history();
        let vocabulary = crate::vocabulary::load_vocabulary();

        let custom_active = crate::llm::client::is_custom_prompt_active(
            config.custom_prompt_enabled,
            &config.custom_prompt,
            &config.language,
        );
        let clipboard = if custom_active && config.custom_prompt.contains("{{clipboard}}") {
            match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                Ok(s) => Some(s),
                Err(e) => {
                    log::warn!("Clipboard read failed: {}; rendering empty", e);
                    None
                }
            }
        } else {
            None
        };

        let opts = crate::llm::client::OptimizeOptions {
            language: &config.language,
            history: &history,
            text_structuring: config.text_structuring,
            structuring_prompt: &config.structuring_prompt,
            vocabulary: &vocabulary,
            source_app: source_app.as_deref(),
            user_tags: &config.user_tags,
            custom_prompt_enabled: config.custom_prompt_enabled,
            custom_prompt: &config.custom_prompt,
            clipboard: clipboard.as_deref(),
        };

        match client.optimize_text_with_options(&text, &opts).await {
            Ok(optimized) => {
                if cancel.is_cancelled() {
                    return Err(AppError::Audio("Cancelled".to_string()));
                }
                // Save history as soon as LLM optimization succeeds (before paste),
                // so context is preserved even if paste fails or is cancelled.
                if let Err(e) = history::append_entry(HistoryEntry {
                    original: text.clone(),
                    corrected: optimized.clone(),
                }) {
                    log::error!("Failed to save transcription history: {}", e);
                }
                optimized
            }
            Err(e) => {
                log::error!("Text optimization failed: {}", e);
                let _ = app.emit(
                    "pipeline-warning",
                    PipelineWarning {
                        message: format!("Text optimization failed: {}. Using original transcription.", e),
                    },
                );
                text.clone()
            }
        }
    } else {
        text.clone()
    };

    if cancel.is_cancelled() {
        return Err(AppError::Audio("Cancelled".to_string()));
    }

    let _ = app.emit(
        "pipeline-state",
        PipelineEvent {
            state: PipelineState::Pasting,
        },
    );

    if !crate::input::check_accessibility() {
        crate::input::request_accessibility();
        return Err(AppError::Input(
            "Accessibility permission required. Please grant access in System Settings → Privacy & Security → Accessibility, then try again.".to_string(),
        ));
    }

    let paste_text_val = final_text.clone();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking(move || paste::paste_text(&paste_text_val)),
    )
        .await
        .map_err(|_| AppError::Input("Paste operation timed out (10s)".to_string()))?
        .map_err(|e| AppError::Input(e.to_string()))??;

    let _ = app.emit(
        "pipeline-state",
        PipelineEvent {
            state: PipelineState::Done {
                transcribed_text: text.clone(),
                text: final_text.clone(),
            },
        },
    );

    let gen = app.state::<OverlayGeneration>().inner().clone();
    let snap = gen.load(Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    hide_overlay_if_current(&app, &gen, snap);

    Ok(final_text)
}

pub type ManagedPipeline = Arc<Mutex<Pipeline>>;

pub fn new_managed(active: PipelineActive) -> ManagedPipeline {
    Arc::new(Mutex::new(Pipeline::new(active)))
}
