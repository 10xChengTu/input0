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
