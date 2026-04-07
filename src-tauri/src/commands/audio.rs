use std::sync::{Arc, Mutex};
#[cfg(not(target_os = "macos"))]
use tauri::Manager;
use tauri::{command, AppHandle, State};
use crate::pipeline::{self, Pipeline};
use crate::errors::AppError;

#[command]
pub async fn start_recording(
    state: State<'_, Arc<Mutex<Pipeline>>>,
    app: AppHandle,
) -> Result<(), AppError> {
    let mut pipeline = state
        .lock()
        .map_err(|e| AppError::Audio(e.to_string()))?;
    pipeline.start_recording(&app)
}

#[command]
pub async fn stop_recording(
    state: State<'_, Arc<Mutex<Pipeline>>>,
    app: AppHandle,
) -> Result<String, AppError> {
    let (recorded, cancel_token) = {
        let mut pipeline = state
            .lock()
            .map_err(|e| AppError::Audio(e.to_string()))?;
        let recorded = pipeline.stop_recording_sync()?;
        let cancel_token = pipeline.cancel_token();
        (recorded, cancel_token)
    };
    pipeline::process_audio(recorded, app, cancel_token).await
}

#[command]
pub async fn toggle_recording(
    state: State<'_, Arc<Mutex<Pipeline>>>,
    app: AppHandle,
) -> Result<bool, AppError> {
    let is_currently_recording = {
        let pipeline = state
            .lock()
            .map_err(|e| AppError::Audio(e.to_string()))?;
        pipeline.is_recording()
    };

    if is_currently_recording {
        let (recorded, cancel_token) = {
            let mut pipeline = state
                .lock()
                .map_err(|e| AppError::Audio(e.to_string()))?;
            let recorded = pipeline.stop_recording_sync()?;
            let cancel_token = pipeline.cancel_token();
            (recorded, cancel_token)
        };
        pipeline::process_audio(recorded, app, cancel_token).await?;
        Ok(false)
    } else {
        let mut pipeline = state
            .lock()
            .map_err(|e| AppError::Audio(e.to_string()))?;
        pipeline.start_recording(&app)?;
        Ok(true)
    }
}

#[command]
pub async fn cancel_pipeline(
    state: State<'_, Arc<Mutex<Pipeline>>>,
    app: AppHandle,
) -> Result<(), AppError> {
    let mut pipeline = state
        .lock()
        .map_err(|e| AppError::Audio(e.to_string()))?;
    pipeline.cancel(&app);
    #[cfg(target_os = "macos")]
    super::window::hide_overlay_async(&app);

    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
    Ok(())
}

#[command]
pub fn list_input_devices() -> Vec<crate::audio::capture::AudioDeviceInfo> {
    crate::audio::capture::list_input_devices()
}

#[command]
pub async fn set_input_device(device_name: String) -> Result<(), AppError> {
    crate::config::update_field("input_device", &device_name)?;
    Ok(())
}
