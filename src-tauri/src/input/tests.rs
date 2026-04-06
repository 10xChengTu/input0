#[cfg(test)]
mod tests {
    use crate::input::hotkey::*;
    use crate::input::paste::*;

    #[test]
    fn test_parse_option_space() {
        let (modifiers, key) = parse_hotkey("Option+Space").unwrap();
        assert_eq!(modifiers, vec!["Option"]);
        assert_eq!(key, "Space");
    }

    #[test]
    fn test_parse_cmd_shift_k() {
        let (modifiers, key) = parse_hotkey("Command+Shift+K").unwrap();
        assert_eq!(modifiers, vec!["Command", "Shift"]);
        assert_eq!(key, "K");
    }

    #[test]
    fn test_parse_single_key() {
        let (modifiers, key) = parse_hotkey("F1").unwrap();
        assert!(modifiers.is_empty());
        assert_eq!(key, "F1");
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse_hotkey("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_modifier() {
        let result = parse_hotkey("Banana+Space");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_trailing_plus() {
        let result = parse_hotkey("Option+");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_spaces() {
        let (modifiers, key) = parse_hotkey(" Option + Space ").unwrap();
        assert_eq!(modifiers, vec!["Option"]);
        assert_eq!(key, "Space");
    }

    #[test]
    fn test_tauri_option_space() {
        let result = to_tauri_shortcut("Option+Space").unwrap();
        assert_eq!(result, "Alt+Space");
    }

    #[test]
    fn test_tauri_cmd_v() {
        let result = to_tauri_shortcut("Command+V").unwrap();
        assert_eq!(result, "Command+V");
    }

    #[test]
    fn test_tauri_ctrl_shift_a() {
        let result = to_tauri_shortcut("Control+Shift+A").unwrap();
        assert_eq!(result, "Control+Shift+A");
    }

    #[test]
    fn test_tauri_case_insensitive() {
        let result = to_tauri_shortcut("option+space").unwrap();
        assert_eq!(result, "Alt+space");
    }

    #[test]
    fn test_tauri_single_key() {
        let result = to_tauri_shortcut("F5").unwrap();
        assert_eq!(result, "F5");
    }

    #[test]
    #[ignore]
    fn test_copy_to_clipboard() {
        let text = "hello clipboard test";
        copy_to_clipboard(text).unwrap();
        let got = get_clipboard_text().unwrap();
        assert_eq!(got, text);
    }

    #[test]
    #[ignore]
    fn test_get_clipboard_text() {
        let text = "get clipboard test";
        copy_to_clipboard(text).unwrap();
        let got = get_clipboard_text().unwrap();
        assert_eq!(got, text);
    }

    #[test]
    #[ignore]
    fn test_clipboard_unicode() {
        let text = "你好世界 🎉";
        copy_to_clipboard(text).unwrap();
        let got = get_clipboard_text().unwrap();
        assert_eq!(got, text);
    }

    #[test]
    #[ignore]
    fn test_clipboard_empty_string() {
        copy_to_clipboard("").unwrap();
        let got = get_clipboard_text().unwrap_or_default();
        assert_eq!(got, "");
    }

    #[test]
    #[ignore]
    fn test_paste_text() {
        paste_text("test paste").unwrap();
    }
}
