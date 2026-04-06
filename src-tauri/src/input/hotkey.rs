use crate::errors::AppError;

pub fn parse_hotkey(hotkey: &str) -> Result<(Vec<String>, String), AppError> {
    let parts: Vec<&str> = hotkey.split('+').collect();
    if parts.is_empty() {
        return Err(AppError::Input("Empty hotkey string".to_string()));
    }

    let key = parts.last().unwrap().trim().to_string();
    let modifiers: Vec<String> = parts[..parts.len() - 1]
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    let valid_modifiers = [
        "Control", "Option", "Alt", "Shift", "Command", "Cmd", "Super",
    ];
    for modifier in &modifiers {
        if !valid_modifiers
            .iter()
            .any(|v| v.eq_ignore_ascii_case(modifier))
        {
            return Err(AppError::Input(format!("Invalid modifier: {}", modifier)));
        }
    }

    if key.is_empty() {
        return Err(AppError::Input("Empty key in hotkey".to_string()));
    }

    Ok((modifiers, key))
}

pub fn to_tauri_shortcut(hotkey: &str) -> Result<String, AppError> {
    let (modifiers, key) = parse_hotkey(hotkey)?;

    let tauri_modifiers: Vec<String> = modifiers
        .iter()
        .map(|m| match m.to_lowercase().as_str() {
            "option" | "alt" => "Alt".to_string(),
            "command" | "cmd" => "Command".to_string(),
            "control" | "ctrl" => "Control".to_string(),
            "shift" => "Shift".to_string(),
            "super" => "Super".to_string(),
            other => other.to_string(),
        })
        .collect();

    if tauri_modifiers.is_empty() {
        Ok(key)
    } else {
        Ok(format!("{}+{}", tauri_modifiers.join("+"), key))
    }
}
