pub mod hotkey;
pub mod paste;
#[cfg(test)]
mod tests;

/// Get the name of the currently frontmost (active) application.
/// Returns `None` on non-macOS platforms or if the app name cannot be determined.
#[cfg(target_os = "macos")]
pub fn get_frontmost_app() -> Option<String> {
    use cocoa::base::{id, nil};
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil {
            return None;
        }
        let name: id = msg_send![app, localizedName];
        if name == nil {
            return None;
        }
        let cstr: *const std::os::raw::c_char = msg_send![name, UTF8String];
        if cstr.is_null() {
            return None;
        }
        Some(
            std::ffi::CStr::from_ptr(cstr)
                .to_string_lossy()
                .into_owned(),
        )
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_frontmost_app() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
mod accessibility {
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) }
    }

    /// Show the macOS system prompt directing the user to grant Accessibility.
    /// The prompt only appears when TCC status is "not determined"; once denied,
    /// the user must enable it manually in System Settings.
    pub fn request_with_prompt() -> bool {
        unsafe {
            use cocoa::base::id;

            let key: id = msg_send![
                class!(NSString),
                stringWithUTF8String: b"AXTrustedCheckOptionPrompt\0".as_ptr()
            ];
            let value: id = msg_send![class!(NSNumber), numberWithBool: true];
            let dict: id = msg_send![
                class!(NSDictionary),
                dictionaryWithObject: value
                forKey: key
            ];
            AXIsProcessTrustedWithOptions(dict as *const c_void)
        }
    }

    pub fn open_settings() {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

#[cfg(target_os = "macos")]
mod microphone {
    use cocoa::base::id;

    pub fn check_permission() -> String {
        unsafe {
            let media_type: id = msg_send![
                class!(NSString),
                stringWithUTF8String: b"soun\0".as_ptr()
            ];
            let status: i64 = msg_send![
                class!(AVCaptureDevice),
                authorizationStatusForMediaType: media_type
            ];
            match status {
                0 => "not_determined".to_string(),
                1 => "restricted".to_string(),
                2 => "denied".to_string(),
                3 => "authorized".to_string(),
                _ => "unknown".to_string(),
            }
        }
    }

    pub fn request_permission() {
        open_settings();
    }

    pub fn open_settings() {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
            .spawn();
    }
}

#[cfg(target_os = "macos")]
pub fn check_accessibility() -> bool {
    accessibility::is_trusted()
}

#[cfg(target_os = "macos")]
pub fn request_accessibility() -> bool {
    accessibility::request_with_prompt()
}

#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() {
    accessibility::open_settings();
}

#[cfg(target_os = "macos")]
pub fn check_microphone_permission() -> String {
    microphone::check_permission()
}

#[cfg(target_os = "macos")]
pub fn request_microphone_permission() {
    microphone::request_permission()
}

#[cfg(target_os = "macos")]
pub fn open_microphone_settings() {
    microphone::open_settings()
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() {}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone_permission() -> String {
    "authorized".to_string()
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone_permission() {}

#[cfg(not(target_os = "macos"))]
pub fn open_microphone_settings() {}
