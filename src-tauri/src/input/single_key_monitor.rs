//! macOS single-modifier push-to-talk monitor backed by CGEventTap.
//!
//! Replaces `fn_monitor.rs`. See docs/feature-single-key-hotkey.md.

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use core_foundation::runloop::{
    kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRef,
    CFRunLoopRun, CFRunLoopSourceRef, CFRunLoopStop,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleKey {
    Fn,
    LeftOption,
    RightOption,
    LeftCommand,
    RightCommand,
    LeftControl,
    RightControl,
    LeftShift,
    RightShift,
}

impl SingleKey {
    /// Parse a raw config string. Accepts `"Fn"`, `"RightOption"`,
    /// `"left_shift"`, `"right-control"`, etc. Case-insensitive.
    pub fn from_raw(raw: &str) -> Option<Self> {
        let normalized: String = raw
            .trim()
            .chars()
            .filter(|c| !matches!(*c, '_' | '-' | ' '))
            .flat_map(|c| c.to_lowercase())
            .collect();
        match normalized.as_str() {
            "fn" => Some(Self::Fn),
            "leftoption" => Some(Self::LeftOption),
            "rightoption" => Some(Self::RightOption),
            "leftcommand" => Some(Self::LeftCommand),
            "rightcommand" => Some(Self::RightCommand),
            "leftcontrol" => Some(Self::LeftControl),
            "rightcontrol" => Some(Self::RightControl),
            "leftshift" => Some(Self::LeftShift),
            "rightshift" => Some(Self::RightShift),
            _ => None,
        }
    }

    /// macOS virtual key code emitted by the physical modifier key.
    pub fn key_code(self) -> i64 {
        match self {
            Self::Fn => 63,
            Self::LeftOption => 58,
            Self::RightOption => 61,
            Self::LeftCommand => 55,
            Self::RightCommand => 54,
            Self::LeftControl => 59,
            Self::RightControl => 62,
            Self::LeftShift => 56,
            Self::RightShift => 60,
        }
    }

    /// Bit in `CGEventGetFlags` that indicates this specific key is held.
    /// Fn uses the device-independent `NSEventModifierFlagFunction`; left/right
    /// split keys use the private `NX_DEVICE*KEYMASK` bits in the low nibble.
    pub fn device_mask(self) -> u64 {
        match self {
            Self::Fn => 1 << 23,            // NSEventModifierFlagFunction
            Self::LeftControl => 0x0000_0001,
            Self::LeftShift => 0x0000_0002,
            Self::RightShift => 0x0000_0004,
            Self::LeftCommand => 0x0000_0008,
            Self::RightCommand => 0x0000_0010,
            Self::LeftOption => 0x0000_0020,
            Self::RightOption => 0x0000_0040,
            Self::RightControl => 0x0000_2000,
        }
    }
}

// ----- CGEventTap raw FFI (core-graphics lacks a usable wrapper here) -----

type CGEventRef = *mut c_void;
type CGEventMask = u64;
type CFMachPortRefRaw = *mut c_void;
type CFAllocatorRef = *const c_void;

#[repr(u32)]
#[allow(dead_code)]
enum CGEventTapLocation {
    HidEventTap = 0,
    SessionEventTap = 1,
    AnnotatedSessionEventTap = 2,
}

#[repr(u32)]
#[allow(dead_code)]
enum CGEventTapPlacement {
    HeadInsertEventTap = 0,
    TailAppendEventTap = 1,
}

#[repr(u32)]
#[allow(dead_code)]
enum CGEventTapOptions {
    Default = 0,
    ListenOnly = 1,
}

#[repr(i64)]
#[allow(dead_code)]
enum CGEventField {
    KeyboardEventKeycode = 9,
}

type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRefRaw;
    fn CGEventTapEnable(tap: CFMachPortRefRaw, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRefRaw,
        order: i64,
    ) -> CFRunLoopSourceRef;
    fn CFMachPortInvalidate(port: CFMachPortRefRaw);
}

type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

struct MonitorContext {
    target_key: SingleKey,
    pressed: AtomicBool,
    callback: Callback,
}

struct MonitorHandles {
    run_loop: CFRunLoopRef,
    thread: Option<JoinHandle<()>>,
    // Heap-allocated via Arc; pointer passed as user_info to CGEventTapCreate.
    context: Arc<MonitorContext>,
    tap_port: CFMachPortRefRaw,
}

// Raw CF pointers are only touched under MONITORS or from the dedicated
// run-loop thread — never shared freely.
unsafe impl Send for MonitorHandles {}

// Lets the handshake channel carry raw CF pointers from the run-loop thread.
struct RawHandles {
    run_loop: CFRunLoopRef,
    tap_port: CFMachPortRefRaw,
}
unsafe impl Send for RawHandles {}

static MONITORS: Mutex<Option<MonitorHandles>> = Mutex::new(None);

unsafe extern "C" fn tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    // Guard against kernel re-disabling on timeout / user input bursts.
    if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        if let Ok(guard) = MONITORS.lock() {
            if let Some(h) = guard.as_ref() {
                CGEventTapEnable(h.tap_port, true);
            }
        }
        return event;
    }

    if event_type != K_CG_EVENT_FLAGS_CHANGED || user_info.is_null() {
        return event;
    }

    let ctx = &*(user_info as *const MonitorContext);
    let key_code = CGEventGetIntegerValueField(event, CGEventField::KeyboardEventKeycode);
    if key_code != ctx.target_key.key_code() {
        return event;
    }

    let flags = CGEventGetFlags(event);
    let now_pressed = (flags & ctx.target_key.device_mask()) != 0;
    let prev = ctx.pressed.swap(now_pressed, Ordering::SeqCst);
    if prev == now_pressed {
        // No true state transition — swallow but don't re-trigger callback.
        return std::ptr::null_mut();
    }

    // Dispatch the user callback off the tap thread so we don't block the tap.
    let cb = ctx.callback.clone();
    std::thread::spawn(move || cb(now_pressed));

    // Consume the event so downstream apps don't see the modifier change.
    std::ptr::null_mut()
}

pub fn start<F>(key: SingleKey, callback: F) -> Result<(), String>
where
    F: Fn(bool) + Send + Sync + 'static,
{
    let context = Arc::new(MonitorContext {
        target_key: key,
        pressed: AtomicBool::new(false),
        callback: Arc::new(callback),
    });

    let mut guard = MONITORS
        .lock()
        .map_err(|e| format!("single_key_monitor mutex poisoned: {}", e))?;
    if guard.is_some() {
        teardown(&mut guard);
    }

    let (tx, rx) = std::sync::mpsc::channel::<Result<RawHandles, String>>();
    let ctx_for_thread = Arc::clone(&context);

    let join = thread::spawn(move || {
        let user_info = Arc::as_ptr(&ctx_for_thread) as *mut c_void;
        let mask: CGEventMask = 1u64 << K_CG_EVENT_FLAGS_CHANGED;
        let tap = unsafe {
            CGEventTapCreate(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                mask,
                tap_callback,
                user_info,
            )
        };
        if tap.is_null() {
            let _ = tx.send(Err(
                "CGEventTapCreate returned NULL — Accessibility permission missing".to_string(),
            ));
            return;
        }

        let source = unsafe { CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0) };
        if source.is_null() {
            unsafe { CFMachPortInvalidate(tap) };
            let _ = tx.send(Err("CFMachPortCreateRunLoopSource returned NULL".to_string()));
            return;
        }

        let run_loop = unsafe { CFRunLoopGetCurrent() };
        unsafe {
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
        }

        let _ = tx.send(Ok(RawHandles { run_loop, tap_port: tap }));
        unsafe { CFRunLoopRun() };
        // Returns only after CFRunLoopStop is called from teardown.
        drop(ctx_for_thread);
    });

    let handles = rx
        .recv()
        .map_err(|e| format!("single_key_monitor thread handshake failed: {}", e))??;

    *guard = Some(MonitorHandles {
        run_loop: handles.run_loop,
        thread: Some(join),
        context,
        tap_port: handles.tap_port,
    });
    Ok(())
}

pub fn stop() -> Result<(), String> {
    let mut guard = MONITORS
        .lock()
        .map_err(|e| format!("single_key_monitor mutex poisoned: {}", e))?;
    teardown(&mut guard);
    Ok(())
}

fn teardown(guard: &mut Option<MonitorHandles>) {
    if let Some(mut handles) = guard.take() {
        unsafe {
            CGEventTapEnable(handles.tap_port, false);
            CFMachPortInvalidate(handles.tap_port);
            CFRunLoopStop(handles.run_loop);
        }
        if let Some(join) = handles.thread.take() {
            let _ = join.join();
        }
        drop(handles.context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_raw_accepts_canonical_names() {
        assert_eq!(SingleKey::from_raw("Fn"), Some(SingleKey::Fn));
        assert_eq!(SingleKey::from_raw("RightOption"), Some(SingleKey::RightOption));
        assert_eq!(SingleKey::from_raw("LeftOption"), Some(SingleKey::LeftOption));
        assert_eq!(SingleKey::from_raw("RightCommand"), Some(SingleKey::RightCommand));
        assert_eq!(SingleKey::from_raw("LeftCommand"), Some(SingleKey::LeftCommand));
        assert_eq!(SingleKey::from_raw("RightControl"), Some(SingleKey::RightControl));
        assert_eq!(SingleKey::from_raw("LeftControl"), Some(SingleKey::LeftControl));
        assert_eq!(SingleKey::from_raw("RightShift"), Some(SingleKey::RightShift));
        assert_eq!(SingleKey::from_raw("LeftShift"), Some(SingleKey::LeftShift));
    }

    #[test]
    fn from_raw_is_case_and_separator_insensitive() {
        assert_eq!(SingleKey::from_raw("fn"), Some(SingleKey::Fn));
        assert_eq!(SingleKey::from_raw("right_option"), Some(SingleKey::RightOption));
        assert_eq!(SingleKey::from_raw("right-option"), Some(SingleKey::RightOption));
        assert_eq!(SingleKey::from_raw("  RIGHT OPTION "), Some(SingleKey::RightOption));
    }

    #[test]
    fn from_raw_rejects_combos_and_unknown() {
        assert_eq!(SingleKey::from_raw("Option+Space"), None);
        assert_eq!(SingleKey::from_raw(""), None);
        assert_eq!(SingleKey::from_raw("Space"), None);
        assert_eq!(SingleKey::from_raw("RightAlt"), None); // we use "Option"
    }

    #[test]
    fn key_codes_are_distinct() {
        let all = [
            SingleKey::Fn,
            SingleKey::LeftOption, SingleKey::RightOption,
            SingleKey::LeftCommand, SingleKey::RightCommand,
            SingleKey::LeftControl, SingleKey::RightControl,
            SingleKey::LeftShift, SingleKey::RightShift,
        ];
        let mut codes: Vec<i64> = all.iter().map(|k| k.key_code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), all.len(), "every SingleKey must have a unique key_code");
    }

    #[test]
    fn device_masks_cover_expected_bits() {
        assert_eq!(SingleKey::Fn.device_mask(), 1 << 23);
        assert_eq!(SingleKey::RightOption.device_mask(), 0x40);
        assert_eq!(SingleKey::LeftCommand.device_mask(), 0x08);
        assert_eq!(SingleKey::RightControl.device_mask(), 0x2000);
    }
}
