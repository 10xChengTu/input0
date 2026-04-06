use crate::errors::AppError;
use arboard::Clipboard;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn paste_text(text: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::Input(format!("Failed to access clipboard: {}", e)))?;

    let previous = clipboard.get_text().unwrap_or_default();

    clipboard
        .set_text(text)
        .map_err(|e| AppError::Input(format!("Failed to set clipboard: {}", e)))?;

    simulate_paste()?;

    thread::sleep(Duration::from_millis(150));

    let _ = clipboard.set_text(&previous);

    Ok(())
}

fn simulate_paste() -> Result<(), AppError> {
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output()
        .map_err(|e| AppError::Input(format!("Failed to run AppleScript: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Input(format!("AppleScript failed: {}", stderr)));
    }

    Ok(())
}

pub fn copy_to_clipboard(text: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::Input(format!("Failed to access clipboard: {}", e)))?;
    clipboard
        .set_text(text)
        .map_err(|e| AppError::Input(format!("Failed to set clipboard: {}", e)))?;
    Ok(())
}

pub fn get_clipboard_text() -> Result<String, AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::Input(format!("Failed to access clipboard: {}", e)))?;
    clipboard
        .get_text()
        .map_err(|e| AppError::Input(format!("Failed to get clipboard: {}", e)))
}
