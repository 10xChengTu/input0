# Single-Key Push-to-Talk Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users bind any single modifier key (Fn / left+right Option / Command / Control / Shift) as push-to-talk, and consume the event so the key's default OS behavior is suppressed.

**Architecture:** Replace the NSEvent-based `fn_monitor.rs` with a CGEventTap-based `single_key_monitor.rs`. CGEventTap runs on a dedicated Core Foundation run-loop thread, intercepts `kCGEventFlagsChanged`, identifies the pressed modifier via `keyCode`, checks its device-specific bit in the event flags to distinguish press/release, and returns `NULL` to consume the event. Config stays a plain string (`"Fn"` / `"RightOption"` / `"Option+Space"` / …); `lib.rs` routes single-key strings to the new monitor and combo strings to `tauri-plugin-global-shortcut` as before.

**Tech Stack:** Rust (core-foundation, core-graphics crates + raw FFI), Tauri v2, React/TypeScript, Zustand.

**Spec:** `docs/feature-single-key-hotkey.md`

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src-tauri/Cargo.toml` | Add `core-foundation` and `core-graphics` deps (macOS target) |
| `src-tauri/src/input/single_key_monitor.rs` | **NEW**. `SingleKey` enum, CGEventTap setup/teardown, press/release callback plumbing. |
| `src-tauri/src/input/fn_monitor.rs` | **DELETE**. Superseded by `single_key_monitor.rs`. |
| `src-tauri/src/input/hotkey.rs` | Add `parse_single_key(raw) -> Option<SingleKey>`. Keep `parse_hotkey` / `to_tauri_shortcut` unchanged. |
| `src-tauri/src/input/mod.rs` | Swap `pub mod fn_monitor;` for `pub mod single_key_monitor;`. |
| `src-tauri/src/lib.rs` | Replace `is_fn_hotkey` checks with `SingleKey::from_raw(..).is_some()`. Route single-key strings to new monitor; combo strings stay on global-shortcut. |
| `src-tauri/src/commands/input.rs` | `update_hotkey` must skip `to_tauri_shortcut` validation for any single key, not just `"Fn"`. |
| `src/components/SettingsPage.tsx` | Dropdown gains a "Single key" group (9 entries) plus a per-option side-effect warning and a permission banner. |
| `src/i18n/types.ts`, `src/i18n/en.ts`, `src/i18n/zh.ts` | Add i18n strings for new preset labels, warnings, and banner. |
| `docs/feature-single-key-hotkey.md` | Update "实现状态" checkboxes at end. |
| `AGENTS.md` (symlinked to `CLAUDE.md`) | Bump "最后校验" date of this feature doc after completion. |

---

## Task 1: Add Core Foundation / Core Graphics Rust bindings

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Edit Cargo.toml to add macOS deps**

Add inside the existing `[target.'cfg(target_os = "macos")'.dependencies]` block (do NOT reformat existing entries):

```toml
core-foundation = "0.10"
core-graphics = "0.24"
```

Final block must read:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.26"
objc = "0.2"
dispatch2 = "0.3"
block = "0.1.6"
core-foundation = "0.10"
core-graphics = "0.24"
```

- [ ] **Step 2: Verify dependencies resolve and compile**

Run: `cd src-tauri && cargo check --lib`
Expected: compiles with no new warnings or errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add core-foundation and core-graphics deps for single-key monitor"
```

---

## Task 2: `SingleKey` enum with keyCode + device-mask mapping (TDD)

**Files:**
- Create: `src-tauri/src/input/single_key_monitor.rs`
- Modify: `src-tauri/src/input/mod.rs`

- [ ] **Step 1: Register new module (macOS only) but keep fn_monitor for now**

Edit `src-tauri/src/input/mod.rs` — only add the new `pub mod` line, do NOT remove `fn_monitor` yet:

```rust
pub mod hotkey;
pub mod paste;
#[cfg(target_os = "macos")]
pub mod fn_monitor;
#[cfg(target_os = "macos")]
pub mod single_key_monitor;
#[cfg(test)]
mod tests;
```

- [ ] **Step 2: Write the failing tests in the new module**

Create `src-tauri/src/input/single_key_monitor.rs` with:

```rust
//! macOS single-modifier push-to-talk monitor backed by CGEventTap.
//!
//! Replaces `fn_monitor.rs`. See docs/feature-single-key-hotkey.md.

#![cfg(target_os = "macos")]

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
```

- [ ] **Step 3: Run tests and confirm they fail to compile (module not yet referenced anywhere else)**

Run: `cd src-tauri && cargo test --lib single_key_monitor::tests -- --nocapture`
Expected: compiles; all 5 tests **PASS** (this task deliberately ships the code + tests together; the "fail first" discipline applies per-behavior, and the enum is pure data with no prior behavior to break).

- [ ] **Step 4: Verify no warnings introduced**

Run: `cd src-tauri && cargo check --lib`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/input/single_key_monitor.rs src-tauri/src/input/mod.rs
git commit -m "feat(input): add SingleKey enum with keyCode and device-mask tables"
```

---

## Task 3: `parse_single_key` helper in `hotkey.rs` (TDD)

Expose a tiny facade so `lib.rs` and `commands/input.rs` don't need to import the platform-gated monitor module.

**Files:**
- Modify: `src-tauri/src/input/hotkey.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/src/input/hotkey.rs`:

```rust
#[cfg(test)]
mod single_key_tests {
    use super::*;

    #[test]
    fn is_single_key_true_for_fn_and_all_split_modifiers() {
        for raw in [
            "Fn", "RightOption", "LeftOption",
            "RightCommand", "LeftCommand",
            "RightControl", "LeftControl",
            "RightShift", "LeftShift",
        ] {
            assert!(is_single_key(raw), "expected single-key: {raw}");
        }
    }

    #[test]
    fn is_single_key_false_for_combos_and_plain_keys() {
        assert!(!is_single_key("Option+Space"));
        assert!(!is_single_key("Command+Shift+R"));
        assert!(!is_single_key("Space"));
        assert!(!is_single_key(""));
    }
}
```

- [ ] **Step 2: Run tests and confirm they fail**

Run: `cd src-tauri && cargo test --lib input::hotkey::single_key_tests`
Expected: FAIL — `is_single_key` is undefined.

- [ ] **Step 3: Implement the function**

Append to `src-tauri/src/input/hotkey.rs` (above the `#[cfg(test)]` block):

```rust
/// Returns true if the raw hotkey string designates a single modifier key
/// handled by the native CGEventTap monitor (as opposed to a normal
/// key-combo handled by `tauri-plugin-global-shortcut`).
pub fn is_single_key(raw: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::input::single_key_monitor::SingleKey::from_raw(raw).is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        raw.trim().eq_ignore_ascii_case("Fn")
    }
}
```

- [ ] **Step 4: Run tests and confirm they pass**

Run: `cd src-tauri && cargo test --lib input::hotkey::single_key_tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/input/hotkey.rs
git commit -m "feat(input): add is_single_key hotkey classifier"
```

---

## Task 4: CGEventTap scaffolding — FFI declarations + background run-loop thread

This task produces an end-to-end `start(key, callback)` / `stop()` API. It is the heaviest task: FFI surface, threading, idempotency. Manual verification comes in Task 9.

**Files:**
- Modify: `src-tauri/src/input/single_key_monitor.rs`

- [ ] **Step 1: Add FFI bindings and types**

Prepend the `SingleKey` block with these imports + FFI declarations (keep the existing enum and tests intact below):

```rust
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use core_foundation::base::TCFType;
use core_foundation::mach_port::{CFMachPort, CFMachPortRef};
use core_foundation::runloop::{
    kCFRunLoopCommonModes, CFRunLoop, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRef,
    CFRunLoopRun, CFRunLoopSourceRef, CFRunLoopStop,
};
use core_foundation::string::CFStringRef;

// ----- CGEventTap raw FFI (core-graphics doesn't expose a safe wrapper) -----

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
```

- [ ] **Step 2: Add shared state + Callback type**

Append below the FFI block:

```rust
type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

struct MonitorContext {
    target_key: SingleKey,
    pressed: AtomicBool,
    callback: Callback,
}

struct MonitorHandles {
    run_loop: CFRunLoopRef,
    thread: Option<JoinHandle<()>>,
    // The context is heap-allocated and its pointer passed to CGEventTap as
    // user_info; we keep an Arc here so Rust ownership drops exactly once on stop().
    context: Arc<MonitorContext>,
    tap_port: CFMachPortRefRaw,
}

// The raw CF pointers are only touched under the MONITORS mutex and from the
// dedicated run-loop thread, never shared freely.
unsafe impl Send for MonitorHandles {}

static MONITORS: Mutex<Option<MonitorHandles>> = Mutex::new(None);
```

- [ ] **Step 3: Implement the CGEventTap C callback**

Append:

```rust
unsafe extern "C" fn tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    // Guard against missed re-enable after tap timeout (CGEventTap is
    // auto-disabled by the kernel if a callback blocks too long).
    if event_type == 0xFFFF_FFFE /* kCGEventTapDisabledByTimeout */
        || event_type == 0xFFFF_FFFF /* kCGEventTapDisabledByUserInput */
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
        // Same-state flagsChanged can fire when modifiers overlap; swallow but
        // don't re-trigger the callback.
        return std::ptr::null_mut();
    }

    // Dispatch off the tap callback to avoid blocking the event tap.
    let cb = ctx.callback.clone();
    std::thread::spawn(move || cb(now_pressed));

    // Consume the event so downstream apps don't see the modifier change.
    std::ptr::null_mut()
}
```

- [ ] **Step 4: Implement `start` / `stop`**

Append:

```rust
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

    // Channel so the spawned thread can hand back the run-loop + tap port
    // once CGEventTapCreate succeeds (or report failure).
    let (tx, rx) = std::sync::mpsc::channel::<Result<(CFRunLoopRef, CFMachPortRefRaw), String>>();
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

        let _ = tx.send(Ok((run_loop, tap)));
        unsafe { CFRunLoopRun() };
        // CFRunLoopRun returns when stop() calls CFRunLoopStop; drop ctx_for_thread here.
        drop(ctx_for_thread);
    });

    let (run_loop, tap_port) = rx
        .recv()
        .map_err(|e| format!("single_key_monitor thread handshake failed: {}", e))??;

    *guard = Some(MonitorHandles {
        run_loop,
        thread: Some(join),
        context,
        tap_port,
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
        // context's last Arc drops here automatically.
        drop(handles.context);
    }
}
```

- [ ] **Step 5: Compile and run unit tests**

Run: `cd src-tauri && cargo test --lib single_key_monitor::tests`
Expected: 5 tests PASS (tests from Task 2 still green; nothing new broke).

Run: `cd src-tauri && cargo check --lib`
Expected: clean build.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/input/single_key_monitor.rs
git commit -m "feat(input): implement CGEventTap-backed single-key monitor"
```

---

## Task 5: Route `lib.rs` to `single_key_monitor` (replace `is_fn_hotkey`)

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace the `is_fn_hotkey` helper**

Find the existing block around `src-tauri/src/lib.rs:55-58`:

```rust
/// Returns true if the given raw hotkey string designates the macOS Fn key.
pub fn is_fn_hotkey(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case("Fn")
}
```

Replace with:

```rust
/// Returns true if the given raw hotkey designates a single modifier key
/// handled by the native CGEventTap monitor.
pub fn is_single_key_hotkey(raw: &str) -> bool {
    input::hotkey::is_single_key(raw)
}

// Back-compat alias; retained only for external callers during migration.
#[deprecated = "use is_single_key_hotkey"]
pub fn is_fn_hotkey(raw: &str) -> bool {
    is_single_key_hotkey(raw)
}
```

- [ ] **Step 2: Rewrite `register_pipeline_shortcut` macOS branch**

Find the block around `src-tauri/src/lib.rs:325-348` (the `if is_fn_hotkey(raw_hotkey)` branch inside `register_pipeline_shortcut`). Replace it with:

```rust
    if is_single_key_hotkey(raw_hotkey) {
        #[cfg(target_os = "macos")]
        {
            use crate::input::single_key_monitor::{self, SingleKey};
            let key = SingleKey::from_raw(raw_hotkey).ok_or_else(|| {
                errors::AppError::Input(format!("Unknown single key: {}", raw_hotkey))
            })?;
            let app_for_cb = app.clone();
            return single_key_monitor::start(key, move |pressed| {
                if pressed {
                    trigger_pipeline_pressed(&app_for_cb);
                } else {
                    trigger_pipeline_released(&app_for_cb);
                }
            })
            .map_err(|e| errors::AppError::Input(
                format!("Failed to start single-key monitor: {}", e),
            ));
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Err(errors::AppError::Input(
                "Single-key hotkeys are only supported on macOS".to_string(),
            ));
        }
    }
```

- [ ] **Step 3: Rewrite `unregister_pipeline_shortcut` macOS branch**

Find the block around `src-tauri/src/lib.rs:367-382`. Replace with:

```rust
    if is_single_key_hotkey(raw_hotkey) {
        #[cfg(target_os = "macos")]
        {
            return crate::input::single_key_monitor::stop().map_err(|e| {
                errors::AppError::Input(format!("Failed to stop single-key monitor: {}", e))
            });
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Ok(());
        }
    }
```

- [ ] **Step 4: Update call sites that still use `is_fn_hotkey`**

Search remaining call sites:

```bash
grep -rn "is_fn_hotkey" src-tauri/src
```

For each hit outside the deprecated alias itself, replace with `is_single_key_hotkey`. Expected hits: `src-tauri/src/lib.rs:660`, `src-tauri/src/commands/input.rs:7` (import) and `input.rs:30`. Update all.

- [ ] **Step 5: Build and test**

Run: `cd src-tauri && cargo test --lib`
Expected: all existing tests PASS.

Run: `cd src-tauri && cargo check --lib`
Expected: clean (no unused-import warnings introduced).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands/input.rs
git commit -m "feat(hotkey): route all single modifier keys through CGEventTap"
```

---

## Task 6: Delete `fn_monitor.rs`

Once Task 5 lands, nothing references `fn_monitor` anymore.

**Files:**
- Delete: `src-tauri/src/input/fn_monitor.rs`
- Modify: `src-tauri/src/input/mod.rs`

- [ ] **Step 1: Confirm no references remain**

Run:
```bash
grep -rn "fn_monitor" src-tauri/src
```
Expected: no results.

- [ ] **Step 2: Remove the module declaration**

Edit `src-tauri/src/input/mod.rs` — delete the two lines:

```rust
#[cfg(target_os = "macos")]
pub mod fn_monitor;
```

- [ ] **Step 3: Delete the file**

```bash
git rm src-tauri/src/input/fn_monitor.rs
```

- [ ] **Step 4: Remove deprecated alias**

Re-edit `src-tauri/src/lib.rs` and delete the `#[deprecated] pub fn is_fn_hotkey` alias added in Task 5 — no callers remain after Task 5 cleaned them up.

- [ ] **Step 5: Build**

Run: `cd src-tauri && cargo test --lib && cargo check --lib`
Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/input/mod.rs src-tauri/src/lib.rs
git commit -m "refactor(input): remove superseded fn_monitor module"
```

---

## Task 7: Expose Accessibility status to the frontend for hotkey registration failures

The existing `check_accessibility_permission` command already exists (`src-tauri/src/commands/input.rs:91`). We just need `update_hotkey` to surface a recognizable error the UI can key off of.

**Files:**
- Modify: `src-tauri/src/commands/input.rs`

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/input/tests.rs` (the module already exists and is the natural home):

```rust
#[test]
fn update_hotkey_accepts_all_single_keys_without_combo_validation() {
    use crate::input::hotkey;
    // Combo validation would choke on bare "RightOption"; confirm we skip it.
    for raw in [
        "Fn", "RightOption", "LeftOption",
        "RightCommand", "LeftCommand",
        "RightControl", "LeftControl",
        "RightShift", "LeftShift",
    ] {
        assert!(hotkey::is_single_key(raw));
        // to_tauri_shortcut must only be called for combo keys; single keys
        // would fail it — guard the order in update_hotkey.
    }
    assert!(!hotkey::is_single_key("Option+Space"));
}
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri && cargo test --lib input::tests`
Expected: PASS (this only exercises `is_single_key`, which was shipped in Task 3).

- [ ] **Step 3: Verify `update_hotkey` is already correct after Task 5**

Re-open `src-tauri/src/commands/input.rs` and confirm the early-return for single keys is in place:

```rust
if !is_single_key_hotkey(&hotkey_str) {
    hotkey::to_tauri_shortcut(&hotkey_str)?;
}
```

If Task 5 replaced `is_fn_hotkey` with `is_single_key_hotkey` here, no further edit is needed. If not, make that edit now.

- [ ] **Step 4: Commit test-only change**

```bash
git add src-tauri/src/input/tests.rs
git commit -m "test(input): cover single-key branches in update_hotkey"
```

---

## Task 8: Frontend — Settings dropdown with single-key group and warnings

**Files:**
- Modify: `src/components/SettingsPage.tsx`
- Modify: `src/i18n/types.ts`
- Modify: `src/i18n/en.ts`
- Modify: `src/i18n/zh.ts`

- [ ] **Step 1: Add i18n keys**

Edit `src/i18n/types.ts` — inside the `settings` interface (near existing `hotkeyPresetFn` line), add:

```ts
    hotkeyPresetRightOption: string;
    hotkeyPresetLeftOption: string;
    hotkeyPresetRightCommand: string;
    hotkeyPresetLeftCommand: string;
    hotkeyPresetRightControl: string;
    hotkeyPresetLeftControl: string;
    hotkeyPresetRightShift: string;
    hotkeyPresetLeftShift: string;
    hotkeySingleKeyGroup: string;
    hotkeyComboGroup: string;
    hotkeySingleKeyWarningRightOption: string;
    hotkeySingleKeyWarningCommand: string;
    hotkeySingleKeyWarningFn: string;
    hotkeyPermissionBannerTitle: string;
    hotkeyPermissionBannerBody: string;
    hotkeyPermissionBannerAction: string;
```

- [ ] **Step 2: Populate English strings**

Edit `src/i18n/en.ts` — inside the `settings:` block (match the existing trailing comma style, do not reorder neighbors):

```ts
    hotkeyPresetRightOption: "Right Option (hold)",
    hotkeyPresetLeftOption: "Left Option (hold)",
    hotkeyPresetRightCommand: "Right Command (hold)",
    hotkeyPresetLeftCommand: "Left Command (hold)",
    hotkeyPresetRightControl: "Right Control (hold)",
    hotkeyPresetLeftControl: "Left Control (hold)",
    hotkeyPresetRightShift: "Right Shift (hold)",
    hotkeyPresetLeftShift: "Left Shift (hold)",
    hotkeySingleKeyGroup: "Single key",
    hotkeyComboGroup: "Combination",
    hotkeySingleKeyWarningRightOption: "Right Option will no longer produce accented characters (e.g. ⌥E → é).",
    hotkeySingleKeyWarningCommand: "Binding Command may break ⌘C / ⌘V while held. Test before relying on it.",
    hotkeySingleKeyWarningFn: "Fn will stop triggering brightness / volume keys while bound.",
    hotkeyPermissionBannerTitle: "Accessibility permission required",
    hotkeyPermissionBannerBody: "Single-key hotkeys need Accessibility access so the key event can be captured and suppressed.",
    hotkeyPermissionBannerAction: "Open System Settings",
```

- [ ] **Step 3: Populate Chinese strings**

Edit `src/i18n/zh.ts` — same keys:

```ts
    hotkeyPresetRightOption: "右 Option（长按）",
    hotkeyPresetLeftOption: "左 Option（长按）",
    hotkeyPresetRightCommand: "右 Command（长按）",
    hotkeyPresetLeftCommand: "左 Command（长按）",
    hotkeyPresetRightControl: "右 Control（长按）",
    hotkeyPresetLeftControl: "左 Control（长按）",
    hotkeyPresetRightShift: "右 Shift（长按）",
    hotkeyPresetLeftShift: "左 Shift（长按）",
    hotkeySingleKeyGroup: "单键",
    hotkeyComboGroup: "组合键",
    hotkeySingleKeyWarningRightOption: "选择后将无法用右 Option 输入重音字符（如 ⌥E → é）。",
    hotkeySingleKeyWarningCommand: "绑定 Command 可能导致按住期间 ⌘C / ⌘V 失效，请先测试。",
    hotkeySingleKeyWarningFn: "绑定后 Fn 将不再触发亮度 / 音量功能行。",
    hotkeyPermissionBannerTitle: "需要辅助功能权限",
    hotkeyPermissionBannerBody: "单键热键需要辅助功能权限，以便捕获并消费按键事件。",
    hotkeyPermissionBannerAction: "打开系统设置",
```

- [ ] **Step 4: Update Settings dropdown in `SettingsPage.tsx`**

Find the `<select>` around `src/components/SettingsPage.tsx:375-410`. Replace its `<option>` children with grouped options (the surrounding `<select>` element and its `onChange` handler stay as-is — that logic already routes any value through `updateHotkey`):

```tsx
<optgroup label={t.settings.hotkeyComboGroup}>
  <option value="Option+Space">{t.settings.hotkeyPresetOptionSpace}</option>
</optgroup>
<optgroup label={t.settings.hotkeySingleKeyGroup}>
  <option value="Fn">{t.settings.hotkeyPresetFn}</option>
  <option value="RightOption">{t.settings.hotkeyPresetRightOption}</option>
  <option value="LeftOption">{t.settings.hotkeyPresetLeftOption}</option>
  <option value="RightCommand">{t.settings.hotkeyPresetRightCommand}</option>
  <option value="LeftCommand">{t.settings.hotkeyPresetLeftCommand}</option>
  <option value="RightControl">{t.settings.hotkeyPresetRightControl}</option>
  <option value="LeftControl">{t.settings.hotkeyPresetLeftControl}</option>
  <option value="RightShift">{t.settings.hotkeyPresetRightShift}</option>
  <option value="LeftShift">{t.settings.hotkeyPresetLeftShift}</option>
</optgroup>
<option value="__custom__">
  {lastCustomHotkey
    ? `${t.settings.hotkeyPresetCustom}: ${lastCustomHotkey}`
    : `${t.settings.hotkeyPresetCustom}…`}
</option>
```

(Preserve the existing `className` on both `<option>` / `<optgroup>` as already applied to neighbors — match the surrounding style tokens.)

- [ ] **Step 5: Update `isPresetHotkey`**

Find line ~185: `const isPresetHotkey = (h: string) => h === "Option+Space" || h === "Fn";`. Replace with:

```tsx
const SINGLE_KEY_PRESETS = [
  "Fn",
  "RightOption", "LeftOption",
  "RightCommand", "LeftCommand",
  "RightControl", "LeftControl",
  "RightShift", "LeftShift",
] as const;
const isPresetHotkey = (h: string) =>
  h === "Option+Space" || (SINGLE_KEY_PRESETS as readonly string[]).includes(h);
```

- [ ] **Step 6: Add inline warning below the dropdown**

Directly beneath the `<select>` (still inside the existing flex container), insert:

```tsx
{hotkey === "RightOption" && (
  <p className="mt-1 text-xs text-[var(--theme-warning)]">
    {t.settings.hotkeySingleKeyWarningRightOption}
  </p>
)}
{(hotkey === "RightCommand" || hotkey === "LeftCommand") && (
  <p className="mt-1 text-xs text-[var(--theme-warning)]">
    {t.settings.hotkeySingleKeyWarningCommand}
  </p>
)}
{hotkey === "Fn" && (
  <p className="mt-1 text-xs text-[var(--theme-warning)]">
    {t.settings.hotkeySingleKeyWarningFn}
  </p>
)}
```

If `--theme-warning` is not defined in the theme tokens, substitute `text-amber-500` — verify against existing warning copy elsewhere in `SettingsPage.tsx` and match that class.

- [ ] **Step 7: Type-check**

Run: `pnpm build`
Expected: `tsc && vite build` both succeed with no errors.

- [ ] **Step 8: Commit**

```bash
git add src/components/SettingsPage.tsx src/i18n/types.ts src/i18n/en.ts src/i18n/zh.ts
git commit -m "feat(ui): expose single-key hotkey presets with per-key warnings"
```

---

## Task 9: Frontend — Accessibility permission banner on start failure

The Rust side already throws `AppError::Input` with `"Failed to start single-key monitor: ... Accessibility permission missing"` when `CGEventTapCreate` returns NULL. Catch that in the settings flow and render a banner with a button invoking the existing `open_accessibility_settings` command.

**Files:**
- Modify: `src/components/SettingsPage.tsx`

- [ ] **Step 1: Add state for permission errors**

Near the other `useState` hooks (around line 184) add:

```tsx
const [accessibilityError, setAccessibilityError] = useState(false);
```

- [ ] **Step 2: Wrap `updateHotkey` calls to detect the failure shape**

Each `updateHotkey` call site (the dropdown `onChange` and the `HotkeyRecorder` `onCapture`) already has a `try / catch`. Inside the `catch` block, add:

```tsx
const message = String(err ?? "");
if (message.toLowerCase().includes("accessibility")) {
  setAccessibilityError(true);
}
```

(Introduce `err` by catching it: `} catch (err) {`.)

- [ ] **Step 3: Render banner above the hotkey row**

Just inside the hotkey card (before the `<h3>` on line ~346), insert:

```tsx
{accessibilityError && (
  <div className="mx-4 mt-4 rounded-md border border-[var(--theme-warning)] bg-[var(--theme-warning-container)] p-3">
    <p className="text-sm font-medium text-[var(--theme-on-warning-container)]">
      {t.settings.hotkeyPermissionBannerTitle}
    </p>
    <p className="mt-1 text-xs text-[var(--theme-on-warning-container)]">
      {t.settings.hotkeyPermissionBannerBody}
    </p>
    <button
      type="button"
      onClick={async () => {
        await invoke("open_accessibility_settings");
        setAccessibilityError(false);
      }}
      className="mt-2 rounded-md bg-[var(--theme-primary)] px-3 py-1 text-xs font-medium text-white hover:bg-[var(--theme-primary-hover)] transition-colors"
    >
      {t.settings.hotkeyPermissionBannerAction}
    </button>
  </div>
)}
```

Ensure `invoke` is already imported (`import { invoke } from "@tauri-apps/api/core";` near the top of the file — verify once).

- [ ] **Step 4: Type-check**

Run: `pnpm build`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add src/components/SettingsPage.tsx
git commit -m "feat(ui): surface Accessibility permission banner when single-key registration fails"
```

---

## Task 10: Manual QA pass

Automated tests cover the pure logic. The CGEventTap path must be exercised by hand.

**Files:** (none — manual)

- [ ] **Step 1: Boot the dev app**

Run: `pnpm tauri dev`

- [ ] **Step 2: Grant permissions**

On first launch, accept the Input Monitoring and Accessibility prompts. If missed, open **System Settings → Privacy & Security → Accessibility** and toggle Input0 on (Accessibility), plus **Input Monitoring** on.

- [ ] **Step 3: Verify each preset round-trips**

For each of the 9 single-key presets in Settings → Hotkey:
1. Select the preset.
2. In another app (e.g. TextEdit), **hold** the key — confirm the overlay appears and recording starts.
3. **Release** — confirm recording stops and transcription begins.

Keep a checklist, one row per key. Any failure → stop and debug; do not proceed.

- [ ] **Step 4: Verify consumption**

Select **Right Option**. Open TextEdit. Type `⌥E` → should **not** produce `é`. Revert to Combination preset; confirm accent input returns.

- [ ] **Step 5: Verify Cmd side effect**

Select **Right Command**. In any app, attempt `⌘C` with your right Command key held. Whatever the observed behavior — working or broken — document it in the spec under "实现状态" → Cmd side effect row, and update the UI warning copy in `SettingsPage.tsx` if it's worse than what the warning currently says.

- [ ] **Step 6: Verify permission banner**

Remove Input0 from System Settings → Privacy & Security → Accessibility. Restart app. Try to select a single-key preset. Expected: toast error and the banner renders with a working "Open System Settings" button.

- [ ] **Step 7: Verify combo hotkey still works**

Switch back to `Option+Space`. Confirm the existing behavior is unchanged (regression guard for the global-shortcut path).

- [ ] **Step 8: Commit manual-QA notes to the feature doc**

Edit `docs/feature-single-key-hotkey.md` — mark the "实现状态" checkboxes as done, and append a short **"QA 结果"** section summarizing Steps 3–7 outcomes (one bullet per key + one for each verification step).

```bash
git add docs/feature-single-key-hotkey.md
git commit -m "docs: record manual QA results for single-key hotkey"
```

---

## Task 11: Refresh documentation index

**Files:**
- Modify: `AGENTS.md` (the file that `CLAUDE.md` symlinks to)

- [ ] **Step 1: Bump "最后校验" date**

Change the line for `docs/feature-single-key-hotkey.md` in the Documentation Map table from `2026-04-25` to the date manual QA was completed (probably the same day, unless QA was deferred).

- [ ] **Step 2: Commit**

```bash
git add AGENTS.md
git commit -m "docs: mark single-key hotkey feature doc as validated"
```

---

## Self-Review Checklist (completed at plan authoring time)

**Spec coverage:**
- ✅ §"技术方案 / 架构" → Tasks 2, 4, 6
- ✅ §"事件识别" → Task 2 (keyCode table), Task 4 (callback)
- ✅ §"事件消费" → Task 4 (`return NULL` paths)
- ✅ §"线程模型" → Task 4 (run-loop thread + handshake)
- ✅ §"权限" → Task 4 (NULL detection), Task 9 (banner)
- ✅ §"lib.rs 路由改造" → Task 5
- ✅ §"配置字符串约定" → Task 2 (`from_raw`)
- ✅ §"前端改动" → Tasks 8, 9
- ✅ §"测试计划" → Tasks 2, 3, 7 (unit), Task 10 (manual)
- ✅ §"文件清单" → covered across tasks

**Placeholder scan:** no TBD / TODO / "similar to" / unshown-code steps found.

**Type consistency:** `SingleKey`, `from_raw`, `key_code`, `device_mask`, `is_single_key`, `is_single_key_hotkey`, `start(key, callback)`, `stop()` appear identically everywhere they are referenced.

**Risks not yet proven:**
- Cmd-while-held: must be measured in Task 10 Step 5; plan has an explicit branch to adjust UI copy.
- CFRunLoop cleanup on thread panic: `teardown` joins the thread after `CFRunLoopStop`; a panic in `tap_callback` is bounded because the callback dispatches the user closure on a fresh thread.
