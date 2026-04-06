use tauri::command;
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use crate::input::{paste, hotkey};
use crate::errors::AppError;
use crate::{config, CurrentShortcut, register_pipeline_shortcut};

#[command]
pub async fn paste_text(text: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        paste::paste_text(&text)
    }).await.map_err(|e| AppError::Input(e.to_string()))?
}

#[command]
pub fn parse_hotkey_string(hotkey_str: String) -> Result<(Vec<String>, String), AppError> {
    hotkey::parse_hotkey(&hotkey_str)
}

#[command]
pub fn get_tauri_shortcut(hotkey_str: String) -> Result<String, AppError> {
    hotkey::to_tauri_shortcut(&hotkey_str)
}

#[command]
pub fn update_hotkey(app: AppHandle, hotkey_str: String) -> Result<(), AppError> {
    let new_tauri_shortcut = hotkey::to_tauri_shortcut(&hotkey_str)?;

    let current_shortcut = app.state::<CurrentShortcut>();
    let old_shortcut = {
        let guard = current_shortcut.lock().map_err(|e| {
            AppError::Input(format!("CurrentShortcut mutex poisoned: {}", e))
        })?;
        guard.clone()
    };

    if old_shortcut == new_tauri_shortcut {
        config::update_field("hotkey", &hotkey_str)?;
        return Ok(());
    }

    let _ = app.global_shortcut().unregister(old_shortcut.as_str());

    if let Err(e) = register_pipeline_shortcut(&app, &new_tauri_shortcut) {
        let _ = register_pipeline_shortcut(&app, &old_shortcut);
        return Err(e);
    }

    config::update_field("hotkey", &hotkey_str)?;

    {
        let mut guard = current_shortcut.lock().map_err(|e| {
            AppError::Input(format!("CurrentShortcut mutex poisoned: {}", e))
        })?;
        *guard = new_tauri_shortcut;
    }

    Ok(())
}

#[command]
pub fn unregister_hotkey(app: AppHandle) -> Result<(), AppError> {
    let current_shortcut = app.state::<CurrentShortcut>();
    let shortcut = {
        let guard = current_shortcut.lock().map_err(|e| {
            AppError::Input(format!("CurrentShortcut mutex poisoned: {}", e))
        })?;
        guard.clone()
    };
    app.global_shortcut()
        .unregister(shortcut.as_str())
        .map_err(|e| AppError::Input(format!("Failed to unregister shortcut '{}': {}", shortcut, e)))
}

#[command]
pub fn reregister_hotkey(app: AppHandle) -> Result<(), AppError> {
    let current_shortcut = app.state::<CurrentShortcut>();
    let shortcut = {
        let guard = current_shortcut.lock().map_err(|e| {
            AppError::Input(format!("CurrentShortcut mutex poisoned: {}", e))
        })?;
        guard.clone()
    };
    register_pipeline_shortcut(&app, &shortcut)
}
