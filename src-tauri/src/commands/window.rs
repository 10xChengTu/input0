use tauri::{command, AppHandle, Manager};
use crate::errors::AppError;

/// Whether we've already swizzled the overlay NSWindow → NSPanel.
/// This must only happen once per window lifetime.
#[cfg(target_os = "macos")]
static PANEL_SWIZZLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// GCD helpers: dispatch a closure synchronously or asynchronously on the main queue.
/// All AppKit / NSWindow / NSPanel / NSView operations MUST go through these helpers
/// to guarantee they execute on the main thread, regardless of which thread the
/// caller is on (Tauri command threads, tokio workers, shortcut callbacks, etc.).
#[cfg(target_os = "macos")]
#[allow(dead_code)]
mod gcd {
    use dispatch2::DispatchQueue;
    use std::sync::{Arc, Mutex};

    /// Run `f` synchronously on the main thread. If already on the main thread,
    /// execute inline to avoid GCD deadlock (dispatch_sync to main queue from
    /// main thread is a classic deadlock).
    pub fn run_on_main_sync<F, R>(f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        if is_main_thread() {
            return f();
        }
        let result: Arc<Mutex<Option<R>>> = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);
        DispatchQueue::main().exec_sync(move || {
            let val = f();
            *result_clone.lock().unwrap() = Some(val);
        });
        Arc::try_unwrap(result)
            .ok()
            .and_then(|m| m.into_inner().ok())
            .flatten()
            .expect("dispatch_sync callback did not execute")
    }

    fn is_main_thread() -> bool {
        extern "C" {
            fn pthread_main_np() -> std::ffi::c_int;
        }
        unsafe { pthread_main_np() != 0 }
    }

    /// Run `f` asynchronously on the main thread (fire-and-forget).
    pub fn run_on_main_async<F>(f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        DispatchQueue::main().exec_async(f);
    }
}

#[cfg(target_os = "macos")]
pub fn gcd_run_on_main_async<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    gcd::run_on_main_async(f);
}

#[cfg(target_os = "macos")]
pub fn gcd_run_on_main_sync<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    gcd::run_on_main_sync(f)
}

/// Pre-warm the overlay window during app setup so the first show is instant.
/// Performs swizzle (NSWindow → NSPanel), transparency, and panel configuration
/// synchronously on the main thread but does NOT make the window visible.
///
/// This eliminates the first-show flash caused by swizzle + AppKit state refresh
/// happening at display time.
#[cfg(target_os = "macos")]
pub fn prewarm_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let ns_window = match window.ns_window() {
            Ok(ptr) => ptr as usize,
            Err(e) => {
                log::error!("prewarm_overlay: failed to get NSWindow: {}", e);
                return;
            }
        };

        // setup runs on the main thread, so gcd::run_on_main_sync will inline-execute.
        gcd::run_on_main_sync(move || {
            unsafe {
                let ns_window = ns_window as cocoa::base::id;

                if !PANEL_SWIZZLED.load(std::sync::atomic::Ordering::SeqCst) {
                    swizzle_to_nspanel(ns_window);
                    PANEL_SWIZZLED.store(true, std::sync::atomic::Ordering::SeqCst);
                }

                configure_panel_for_fullscreen(ns_window);
                ensure_window_transparency(ns_window);
                // Deliberately NOT calling orderFrontRegardless — window stays hidden.
            }
        });
    }
}

pub fn position_and_show_overlay(app: &AppHandle) {
    crate::register_escape_shortcut(app);

    if let Some(window) = app.get_webview_window("overlay") {
        #[cfg(target_os = "macos")]
        {
            let ns_window = match window.ns_window() {
                Ok(ptr) => ptr as usize, // store as usize for Send
                Err(e) => {
                    log::error!("Failed to get NSWindow for overlay: {}", e);
                    return;
                }
            };

            // PERF: async dispatch avoids blocking tokio worker + main thread starvation
            gcd::run_on_main_async(move || {
                unsafe {
                    use cocoa::base::id;
                    use cocoa::appkit::NSScreen;
                    use cocoa::foundation::{NSArray, NSRect, NSPoint, NSSize};

                    let ns_window = ns_window as id;

                    if !PANEL_SWIZZLED.load(std::sync::atomic::Ordering::SeqCst) {
                        swizzle_to_nspanel(ns_window);
                        PANEL_SWIZZLED.store(true, std::sync::atomic::Ordering::SeqCst);
                    }

                    let mouse_loc: NSPoint = msg_send![class!(NSEvent), mouseLocation];

                    let screens: id = NSScreen::screens(std::ptr::null_mut());
                    let count = screens.count() as usize;

                    let mut target_frame: Option<NSRect> = None;
                    for i in 0..count {
                        let screen: id = screens.objectAtIndex(i as u64);
                        let frame: NSRect = msg_send![screen, frame];
                        if mouse_loc.x >= frame.origin.x
                            && mouse_loc.x < frame.origin.x + frame.size.width
                            && mouse_loc.y >= frame.origin.y
                            && mouse_loc.y < frame.origin.y + frame.size.height
                        {
                            target_frame = Some(frame);
                            break;
                        }
                    }

                    if target_frame.is_none() && count > 0 {
                        let primary: id = screens.objectAtIndex(0);
                        let frame: NSRect = msg_send![primary, frame];
                        target_frame = Some(frame);
                    }

                    if let Some(screen_frame) = target_frame {
                        let overlay_w: f64 = 800.0;
                        let overlay_h: f64 = 200.0;
                        let x = screen_frame.origin.x + (screen_frame.size.width - overlay_w) / 2.0;
                        let y = screen_frame.origin.y;

                        let window_frame = NSRect::new(
                            NSPoint::new(x, y),
                            NSSize::new(overlay_w, overlay_h),
                        );
                        let _: () = msg_send![ns_window, setFrame: window_frame display: true];
                    }

                    configure_panel_for_fullscreen(ns_window);
                    ensure_window_transparency(ns_window);

                    let _: () = msg_send![ns_window, orderFrontRegardless];
                }
            });
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = window.show();
        }
    }
}

/// Swizzle NSWindow's isa pointer to NSPanel at runtime.
/// This makes the window respond to NSPanel-specific behaviors, most critically
/// `NSWindowStyleMaskNonactivatingPanel` which is required for appearing above
/// fullscreen macOS apps.
///
/// Reference: screenpipe uses this exact approach in production.
#[cfg(target_os = "macos")]
unsafe fn swizzle_to_nspanel(ns_window: cocoa::base::id) {
    use objc::runtime::Class;

    extern "C" {
        fn object_setClass(
            obj: *mut objc::runtime::Object,
            cls: *const objc::runtime::Class,
        ) -> *const objc::runtime::Class;
    }

    if let Some(nspanel_class) = Class::get("NSPanel") {
        object_setClass(ns_window as *mut _, nspanel_class);
    }

    // Add NSWindowStyleMaskNonactivatingPanel (bit 7 = 128) to styleMask.
    // This is the key bit that allows the panel to float above fullscreen Spaces
    // without stealing focus or activating our app.
    let current_mask: u64 = msg_send![ns_window, styleMask];
    let non_activating_panel_mask: u64 = 1 << 7; // NSWindowStyleMaskNonactivatingPanel
    let new_mask = current_mask | non_activating_panel_mask;
    let _: () = msg_send![ns_window, setStyleMask: new_mask];
}

/// Configure the swizzled NSPanel window properties for fullscreen overlay display.
#[cfg(target_os = "macos")]
unsafe fn configure_panel_for_fullscreen(ns_window: cocoa::base::id) {
    let _: () = msg_send![ns_window, setLevel: 1000_i64];

    // Collection behavior:
    //   NSWindowCollectionBehaviorCanJoinAllSpaces  = 1 << 0 = 1
    //   NSWindowCollectionBehaviorStationary        = 1 << 4 = 16
    //   NSWindowCollectionBehaviorFullScreenAuxiliary = 1 << 8 = 256
    //   NSWindowCollectionBehaviorIgnoresCycle       = 1 << 6 = 64
    // Total = 1 | 16 | 256 | 64 = 337
    let behavior: u64 = 1 | 16 | 256 | 64;
    let _: () = msg_send![ns_window, setCollectionBehavior: behavior];

    let _: () = msg_send![ns_window, setHidesOnDeactivate: false];
}

#[cfg(target_os = "macos")]
unsafe fn ensure_window_transparency(ns_window: cocoa::base::id) {
    use cocoa::appkit::NSColor;

    let _: () = msg_send![ns_window, setOpaque: false];
    let clear = NSColor::clearColor(cocoa::base::nil);
    let _: () = msg_send![ns_window, setBackgroundColor: clear];
    let _: () = msg_send![ns_window, setHasShadow: false];
}

pub fn hide_overlay_async(app: &AppHandle) {
    crate::unregister_escape_shortcut(app);

    #[cfg(target_os = "macos")]
    if let Some(window) = app.get_webview_window("overlay") {
        if let Ok(ns_window) = window.ns_window() {
            let ptr = ns_window as usize;
            gcd::run_on_main_async(move || {
                unsafe {
                    let ns_window = ptr as cocoa::base::id;
                    let _: () = msg_send![ns_window, orderOut: cocoa::base::nil];
                }
            });
        }
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
}

#[command]
pub async fn show_overlay(app: AppHandle) -> Result<(), AppError> {
    position_and_show_overlay(&app);
    Ok(())
}

#[command]
pub async fn hide_overlay(app: AppHandle) -> Result<(), AppError> {
    hide_overlay_async(&app);
    Ok(())
}

#[command]
pub async fn set_window_theme(app: AppHandle, dark: bool) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        if let Some(window) = app.get_webview_window("main") {
            let ns_window = window
                .ns_window()
                .map_err(|e| AppError::Input(e.to_string()))? as usize;

            gcd::run_on_main_sync(move || {
                unsafe {
                    use cocoa::appkit::{NSColor, NSWindow};
                    use cocoa::base::{id, nil};

                    let ns_window = ns_window as id;

                    let appearance_name: &[u8] = if dark {
                        b"NSAppearanceNameDarkAqua\0"
                    } else {
                        b"NSAppearanceNameAqua\0"
                    };
                    let name_nsstring: id = msg_send![
                        class!(NSString),
                        stringWithUTF8String: appearance_name.as_ptr() as *const std::ffi::c_char
                    ];
                    let appearance: id =
                        msg_send![class!(NSAppearance), appearanceNamed: name_nsstring];
                    let _: () = msg_send![ns_window, setAppearance: appearance];

                    let (r, g, b) = if dark {
                        (0x12 as f64, 0x12 as f64, 0x12 as f64)
                    } else {
                        (0xf9 as f64, 0xf9 as f64, 0xf9 as f64)
                    };
                    let bg_color = NSColor::colorWithRed_green_blue_alpha_(
                        nil,
                        r / 255.0,
                        g / 255.0,
                        b / 255.0,
                        1.0,
                    );
                    ns_window.setBackgroundColor_(bg_color);
                }
            });
        }
    }
    Ok(())
}

#[command]
pub async fn show_settings(app: AppHandle) -> Result<(), AppError> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| AppError::Input(e.to_string()))?;
        window.set_focus().map_err(|e| AppError::Input(e.to_string()))?;
    }
    Ok(())
}
