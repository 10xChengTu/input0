#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

pub mod audio;
pub mod commands;
pub mod config;
pub mod errors;
pub mod history;
pub mod input;
pub mod llm;
pub mod models;
pub mod pipeline;
pub mod stt;
pub mod vocabulary;
pub mod whisper;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::models::registry::{self, BackendKind};
use crate::models::manager as model_manager;
use crate::pipeline::OverlayGeneration;
use crate::stt::{SharedTranscriber, whisper_backend::WhisperBackend, sensevoice_backend::SenseVoiceBackend, paraformer_backend::ParaformerBackend, moonshine_backend::MoonshineBackend, fire_red_asr_backend::FireRedAsrBackend, zipformer_ctc_backend::ZipformerCtcBackend};

/// Managed state: the raw hotkey string currently registered.
/// Stores the user-facing format (e.g. "Option+Space" or "Fn") so we can
/// route press/release events to the appropriate backend (global shortcut
/// plugin vs. native Fn monitor).
pub type CurrentShortcut = Arc<Mutex<String>>;

/// Whether the global Escape shortcut is currently registered. Used to
/// make register/unregister idempotent and to keep ESC only captured while
/// the overlay is visible — otherwise it would steal ESC from every other
/// app in the system.
static ESCAPE_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Managed state: set to true when a pipeline start failed between Pressed
/// and Released, so the Released handler knows to schedule the error toast
/// hide instead of calling stop_recording.
///
/// Wrapped in a newtype struct because Tauri's `manage()` keys state by
/// concrete type. `PipelineActive` is also `Arc<AtomicBool>`, so using a bare
/// type alias here would collide and panic at startup.
pub struct PipelineErrorPending(pub Arc<AtomicBool>);

pub fn new_pipeline_error_pending() -> PipelineErrorPending {
    PipelineErrorPending(Arc::new(AtomicBool::new(false)))
}

/// Returns true if the given raw hotkey designates a single modifier key
/// handled by the native CGEventTap monitor.
pub fn is_single_key_hotkey(raw: &str) -> bool {
    input::hotkey::is_single_key(raw)
}

/// Handle the "pressed" edge of the activation hotkey. Invoked from both the
/// global-shortcut callback and the native Fn key monitor.
pub fn trigger_pipeline_pressed(app: &tauri::AppHandle) {
    let pipeline_arc = app.state::<Arc<Mutex<pipeline::Pipeline>>>().inner().clone();
    let overlay_gen = app.state::<OverlayGeneration>().inner().clone();
    let pipeline_active = app.state::<pipeline::PipelineActive>().inner().clone();
    let error_pending = app.state::<PipelineErrorPending>().0.clone();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if pipeline_active.load(Ordering::SeqCst) {
            return;
        }
        overlay_gen.fetch_add(1, Ordering::SeqCst);
        let source_app = crate::input::get_frontmost_app();
        let pa = Arc::clone(&pipeline_arc);
        let ah = app.clone();
        let ah_overlay = app.clone();
        let ah_err = app.clone();
        let ep = Arc::clone(&error_pending);
        tauri::async_runtime::spawn(async move {
            commands::window::position_and_show_overlay(&ah_overlay);

            let _ = ah_overlay.emit(
                "pipeline-state",
                pipeline::PipelineEvent {
                    state: pipeline::PipelineState::Recording,
                },
            );

            let start_result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                tokio::task::spawn_blocking(move || match pa.lock() {
                    Ok(mut p) => {
                        p.set_source_app(source_app);
                        p.start_recording(&ah)
                    }
                    Err(e) => Err(crate::errors::AppError::Audio(
                        format!("Mutex poisoned: {}", e),
                    )),
                }),
            )
            .await;

            let err_msg = match start_result {
                Ok(Ok(Ok(()))) => None,
                Ok(Ok(Err(e))) => Some(e.to_string()),
                Ok(Err(e)) => Some(format!("spawn_blocking panic: {}", e)),
                Err(_) => Some("start_recording timed out (10s)".to_string()),
            };

            if let Some(msg) = err_msg {
                log::error!("Pipeline start_recording error: {}", msg);
                let _ = ah_err.emit(
                    "pipeline-state",
                    pipeline::PipelineEvent {
                        state: pipeline::PipelineState::Error { message: msg },
                    },
                );
                ep.store(true, Ordering::SeqCst);
            }
        });
    }));

    if let Err(e) = result {
        log::error!(
            "CRITICAL: panic in hotkey pressed handler caught: {:?}",
            e.downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic")
        );
    }
}

/// Handle the "released" edge of the activation hotkey.
pub fn trigger_pipeline_released(app: &tauri::AppHandle) {
    let pipeline_arc = app.state::<Arc<Mutex<pipeline::Pipeline>>>().inner().clone();
    let overlay_gen = app.state::<OverlayGeneration>().inner().clone();
    let pipeline_active = app.state::<pipeline::PipelineActive>().inner().clone();
    let error_pending = app.state::<PipelineErrorPending>().0.clone();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if error_pending.swap(false, Ordering::SeqCst) {
            let ah = app.clone();
            let gen = Arc::clone(&overlay_gen);
            let snap = gen.load(Ordering::SeqCst);
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
                pipeline::hide_overlay_if_current(&ah, &gen, snap);
            });
            return;
        }
        if !pipeline_active.load(Ordering::SeqCst) {
            return;
        }
        let pa = Arc::clone(&pipeline_arc);
        let ah = app.clone();
        let gen = Arc::clone(&overlay_gen);
        tauri::async_runtime::spawn(async move {
            let stop_result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                tokio::task::spawn_blocking(move || match pa.lock() {
                    Ok(mut p) => {
                        let ct = p.cancel_token();
                        let recorded = p.stop_recording_sync()?;
                        Ok((recorded, ct))
                    }
                    Err(e) => Err(crate::errors::AppError::Audio(
                        format!("Mutex poisoned: {}", e),
                    )),
                }),
            )
            .await;

            let (recorded, cancel_token) = match stop_result {
                Ok(Ok(Ok(r))) => r,
                Ok(Ok(Err(e))) => {
                    log::error!("stop_recording_sync error: {}", e);
                    let _ = ah.emit(
                        "pipeline-state",
                        pipeline::PipelineEvent {
                            state: pipeline::PipelineState::Error {
                                message: e.to_string(),
                            },
                        },
                    );
                    let snap = gen.load(Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
                    pipeline::hide_overlay_if_current(&ah, &gen, snap);
                    return;
                }
                Ok(Err(e)) => {
                    log::error!("stop spawn_blocking panic: {}", e);
                    return;
                }
                Err(_) => {
                    log::error!("stop_recording timed out (10s)");
                    let _ = ah.emit(
                        "pipeline-state",
                        pipeline::PipelineEvent {
                            state: pipeline::PipelineState::Error {
                                message: "stop_recording timed out".to_string(),
                            },
                        },
                    );
                    let snap = gen.load(Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
                    pipeline::hide_overlay_if_current(&ah, &gen, snap);
                    return;
                }
            };

            match pipeline::process_audio(recorded, ah.clone(), cancel_token).await {
                Err(e) if e.to_string().contains("Cancelled") => {
                    log::info!("Pipeline cancelled by user");
                }
                Err(e) => {
                    log::error!("process_audio error: {}", e);
                    let _ = ah.emit(
                        "pipeline-state",
                        pipeline::PipelineEvent {
                            state: pipeline::PipelineState::Error {
                                message: e.to_string(),
                            },
                        },
                    );
                    let snap = gen.load(Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
                    pipeline::hide_overlay_if_current(&ah, &gen, snap);
                }
                Ok(_) => {}
            }
        });
    }));

    if let Err(e) = result {
        log::error!(
            "CRITICAL: panic in hotkey released handler caught: {:?}",
            e.downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic")
        );
    }
}

/// Register the global Escape shortcut used to cancel the active pipeline
/// and hide the overlay. This must only be called while the overlay is
/// visible — otherwise ESC gets captured from every other app on the system.
/// Idempotent: safe to call when already registered.
pub fn register_escape_shortcut(app: &tauri::AppHandle) {
    if ESCAPE_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }

    let esc_pipeline_arc = app.state::<Arc<Mutex<pipeline::Pipeline>>>().inner().clone();
    let esc_app_handle = app.clone();

    let res = app.global_shortcut().on_shortcut(
        "Escape",
        move |_app_handle, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            let pa = Arc::clone(&esc_pipeline_arc);
            let ah = esc_app_handle.clone();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tauri::async_runtime::spawn(async move {
                    commands::window::hide_overlay_async(&ah);

                    let ah_cancel = ah.clone();
                    let cancel_result = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        tokio::task::spawn_blocking(move || {
                            match pa.lock() {
                                Ok(mut p) => {
                                    p.cancel(&ah_cancel);
                                }
                                Err(e) => {
                                    log::error!("Mutex poisoned (cancel): {}", e);
                                }
                            }
                        }),
                    )
                    .await;

                    if let Err(_) = cancel_result {
                        log::error!("cancel pipeline timed out (5s)");
                    }
                });
            }));

            if let Err(e) = result {
                log::error!(
                    "CRITICAL: panic in ESC handler caught: {:?}",
                    e.downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| e.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic")
                );
            }
        },
    );

    if let Err(e) = res {
        ESCAPE_REGISTERED.store(false, Ordering::SeqCst);
        log::error!("Failed to register Escape shortcut: {}", e);
    }
}

/// Unregister the global Escape shortcut. Idempotent.
pub fn unregister_escape_shortcut(app: &tauri::AppHandle) {
    if !ESCAPE_REGISTERED.swap(false, Ordering::SeqCst) {
        return;
    }
    if let Err(e) = app.global_shortcut().unregister("Escape") {
        log::warn!("Failed to unregister Escape shortcut: {}", e);
    }
}

/// Register a pipeline activation hotkey, routing to the global-shortcut
/// plugin for normal key combos or to the native macOS Fn monitor when the
/// raw hotkey is "Fn".
pub fn register_pipeline_shortcut(
    app: &tauri::AppHandle,
    raw_hotkey: &str,
) -> Result<(), errors::AppError> {
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

    let tauri_shortcut = input::hotkey::to_tauri_shortcut(raw_hotkey)?;

    app.global_shortcut()
        .on_shortcut(tauri_shortcut.as_str(), move |app_handle, _shortcut, event| {
            match event.state {
                ShortcutState::Pressed => trigger_pipeline_pressed(app_handle),
                ShortcutState::Released => trigger_pipeline_released(app_handle),
            }
        })
        .map_err(|e| {
            errors::AppError::Input(format!(
                "Failed to register shortcut '{}': {}",
                tauri_shortcut, e
            ))
        })
}

/// Unregister whichever backend currently holds the given raw hotkey.
pub fn unregister_pipeline_shortcut(
    app: &tauri::AppHandle,
    raw_hotkey: &str,
) -> Result<(), errors::AppError> {
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

    let tauri_shortcut = input::hotkey::to_tauri_shortcut(raw_hotkey)?;
    app.global_shortcut()
        .unregister(tauri_shortcut.as_str())
        .map_err(|e| {
            errors::AppError::Input(format!(
                "Failed to unregister shortcut '{}': {}",
                tauri_shortcut, e
            ))
        })
}

fn load_stt_model(transcriber: &SharedTranscriber, model_id: &str) -> Result<(), errors::AppError> {
    let info = registry::get_model(model_id)
        .ok_or_else(|| errors::AppError::Whisper(format!("Unknown STT model: {}", model_id)))?;

    if !model_manager::is_model_downloaded(model_id) {
        return Err(errors::AppError::Whisper(format!(
            "Model '{}' is not downloaded yet",
            model_id
        )));
    }

    let backend: Box<dyn stt::TranscriberBackend> = match info.backend {
        BackendKind::Whisper => {
            let path = model_manager::whisper_model_path(model_id)?;
            Box::new(WhisperBackend::new(&path, model_id)?)
        }
        BackendKind::SenseVoice => {
            let (onnx, tokens) = model_manager::sensevoice_model_paths(model_id)?;
            Box::new(SenseVoiceBackend::new(&onnx, &tokens, model_id)?)
        }
        BackendKind::Paraformer => {
            let (onnx, tokens) = model_manager::paraformer_model_paths(model_id)?;
            Box::new(ParaformerBackend::new(&onnx, &tokens, model_id)?)
        }
        BackendKind::Moonshine => {
            let (preprocessor, encoder, uncached_decoder, cached_decoder, tokens) =
                model_manager::moonshine_model_paths(model_id)?;
            Box::new(MoonshineBackend::new(
                &preprocessor, &encoder, &uncached_decoder, &cached_decoder, &tokens, model_id,
            )?)
        }
        BackendKind::FireRedAsr => {
            let (encoder, decoder, tokens) =
                model_manager::fire_red_asr_model_paths(model_id)?;
            Box::new(FireRedAsrBackend::new(&encoder, &decoder, &tokens, model_id)?)
        }
        BackendKind::ZipformerCtc => {
            let (model, tokens) = model_manager::zipformer_ctc_model_paths(model_id)?;
            Box::new(ZipformerCtcBackend::new(&model, &tokens, model_id)?)
        }
    };

    let mut guard = transcriber.lock().map_err(|e| {
        errors::AppError::Whisper(format!("Transcriber mutex poisoned: {}", e))
    })?;
    guard.load(backend);
    Ok(())
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown payload".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let crash_msg = format!(
            "PANIC in thread '{}' at {}: {}\n\nTimestamp: {:?}\n",
            thread_name,
            location,
            payload,
            std::time::SystemTime::now(),
        );

        eprintln!("FATAL: {}", crash_msg);

        if let Some(home) = dirs::home_dir() {
            let log_dir = home.join("Library/Logs/Input0");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_file = log_dir.join("crash.log");
            let _ = std::fs::write(&log_file, &crash_msg);
        }

        default_hook(info);

        std::process::exit(1);
    }));
}

/// Watchdog: spawns a background thread that pings the main thread every 15s.
/// If the main thread doesn't respond within 30s, force-exit with code 42
/// so the user can simply reopen the app instead of rebooting.
#[cfg(target_os = "macos")]
fn start_main_thread_watchdog() {
    use std::sync::atomic::{AtomicU64, Ordering};

    static WATCHDOG_COUNTER: AtomicU64 = AtomicU64::new(0);

    std::thread::Builder::new()
        .name("watchdog".into())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));

            loop {
                let before = WATCHDOG_COUNTER.load(Ordering::SeqCst);

                commands::window::gcd_run_on_main_async(move || {
                    WATCHDOG_COUNTER.fetch_add(1, Ordering::SeqCst);
                });

                std::thread::sleep(std::time::Duration::from_secs(15));

                let after = WATCHDOG_COUNTER.load(Ordering::SeqCst);
                if after == before {
                    eprintln!(
                        "WATCHDOG: main thread unresponsive for >15s, force-exiting (code 42)"
                    );
                    if let Some(home) = dirs::home_dir() {
                        let log_dir = home.join("Library/Logs/Input0");
                        let _ = std::fs::create_dir_all(&log_dir);
                        let log_file = log_dir.join("crash.log");
                        let _ = std::fs::write(
                            &log_file,
                            format!(
                                "WATCHDOG: main thread hang detected at {:?}\n",
                                std::time::SystemTime::now()
                            ),
                        );
                    }
                    std::process::exit(42);
                }
            }
        })
        .expect("Failed to spawn watchdog thread");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();

    #[cfg(target_os = "macos")]
    start_main_thread_watchdog();

    let pipeline_active = pipeline::new_pipeline_active();
    let pipeline_active_for_manage = pipeline_active.clone();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stderr),
                ])
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(pipeline::new_managed(pipeline_active_for_manage))
        .manage(pipeline_active.clone())
        .manage(pipeline::new_overlay_generation())
        .manage(new_pipeline_error_pending())
        .manage(stt::new_shared_transcriber())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            #[allow(deprecated)]
            {
                use cocoa::appkit::{NSColor, NSWindow};
                use cocoa::base::{id, nil};

                if let Some(window) = app.get_webview_window("main") {
                    let ns_window = window.ns_window().unwrap() as id;
                    unsafe {
                        // Sync with --theme-surface in src/index.css
                        let is_dark: bool = {
                            let appearance: id = objc::msg_send![ns_window, effectiveAppearance];
                            let name: id = objc::msg_send![appearance, name];
                            let desc: id = objc::msg_send![name, description];
                            let cstr = std::ffi::CStr::from_ptr(objc::msg_send![desc, UTF8String]);
                            cstr.to_string_lossy().contains("Dark")
                        };
                        let (r, g, b) = if is_dark {
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
                }
            }

            #[cfg(target_os = "macos")]
            commands::window::prewarm_overlay(app.handle());

            let config = config::load().unwrap_or_default();

            let transcriber = app.state::<SharedTranscriber>().inner().clone();

            // Try loading the configured STT model.
            // Fallback chain: config.stt_model → legacy model_path → skip
            let stt_model_id = config.stt_model.clone();
            if model_manager::is_model_downloaded(&stt_model_id) {
                if let Err(e) = load_stt_model(&transcriber, &stt_model_id) {
                    log::error!("Failed to load STT model '{}': {}", stt_model_id, e);
                }
            } else if !config.model_path.is_empty() {
                // Legacy: user has a custom model_path configured
                let legacy_path = std::path::PathBuf::from(&config.model_path);
                if legacy_path.exists() {
                    log::info!("Loading legacy whisper model from: {:?}", legacy_path);
                    match WhisperBackend::new(&legacy_path, "whisper-custom") {
                        Ok(backend) => {
                            if let Ok(mut guard) = transcriber.lock() {
                                guard.load(Box::new(backend));
                            }
                        }
                        Err(e) => log::error!("Failed to load legacy whisper model: {}", e),
                    }
                }
            } else {
                // Dev-mode fallback: look for bundled model in resources
                let resource_path = app
                    .path()
                    .resource_dir()
                    .ok();
                let dev_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("resources")
                    .join("ggml-base.bin");

                let model_path = resource_path
                    .map(|r| r.join("ggml-base.bin"))
                    .filter(|p| p.exists())
                    .or_else(|| if dev_fallback.exists() { Some(dev_fallback) } else { None });

                if let Some(path) = model_path {
                    log::info!("Loading bundled whisper model from: {:?}", path);
                    match WhisperBackend::new(&path, "whisper-base") {
                        Ok(backend) => {
                            if let Ok(mut guard) = transcriber.lock() {
                                guard.load(Box::new(backend));
                            }
                        }
                        Err(e) => log::error!("Failed to load bundled whisper model: {}", e),
                    }
                } else {
                    log::warn!("No STT model available. Please download a model in Settings.");
                }
            }

            let raw_hotkey = config.hotkey.clone();
            if !is_single_key_hotkey(&raw_hotkey) {
                if let Err(e) = input::hotkey::to_tauri_shortcut(&raw_hotkey) {
                    log::warn!("Invalid hotkey config '{}': {}", raw_hotkey, e);
                }
            }

            let app_handle = app.handle().clone();
            app.manage(Arc::new(Mutex::new(raw_hotkey.clone())) as CurrentShortcut);

            if let Err(e) = register_pipeline_shortcut(&app_handle, &raw_hotkey) {
                log::error!("Failed to register activation hotkey '{}': {}", raw_hotkey, e);
            }

            let settings_item = MenuItem::with_id(app, "settings", "Show Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Input0", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            let tray_icon_bytes = include_bytes!("../icons/tray-icon@2x.png");
            let tray_icon = tauri::image::Image::from_bytes(tray_icon_bytes)
                .expect("Failed to load tray icon");

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app_handle, event| {
                    match event.id.as_ref() {
                        "settings" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::save_config,
            commands::config::update_config_field,
            commands::llm::optimize_text,
            commands::llm::test_api_connection,
            commands::llm::get_default_prompt_template,
            commands::llm::get_default_structuring_module,
            commands::whisper::transcribe_audio,
            commands::whisper::init_whisper_model,
            commands::whisper::is_whisper_model_loaded,
            commands::input::paste_text,
            commands::input::parse_hotkey_string,
            commands::input::get_tauri_shortcut,
            commands::input::update_hotkey,
            commands::input::unregister_hotkey,
            commands::input::reregister_hotkey,
            commands::input::check_accessibility_permission,
            commands::input::request_accessibility_permission,
            commands::input::open_accessibility_settings,
            commands::input::check_microphone_permission,
            commands::input::request_microphone_permission,
            commands::input::open_microphone_settings,
            commands::audio::list_input_devices,
            commands::audio::set_input_device,
            commands::audio::start_recording,
            commands::audio::stop_recording,
            commands::audio::toggle_recording,
            commands::audio::cancel_pipeline,
            commands::window::show_overlay,
            commands::window::hide_overlay,
            commands::window::show_settings,
            commands::window::set_window_theme,
            commands::models::list_models,
            commands::models::download_model,
            commands::models::switch_model,
            commands::models::delete_model,
            commands::models::get_model_recommendation,
            commands::vocabulary::get_vocabulary,
            commands::vocabulary::add_vocabulary_entry,
            commands::vocabulary::remove_vocabulary_entry,
            commands::vocabulary::validate_and_add_vocabulary,
            commands::vocabulary::set_vocabulary,
            commands::data::export_data_to_file,
            commands::data::import_data_from_file,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { has_visible_windows, .. } = event {
                if !has_visible_windows {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });
}
