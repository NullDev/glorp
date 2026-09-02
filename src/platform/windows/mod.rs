pub mod app;
pub mod blocklist;
pub mod constants;
pub mod flaglist;
pub mod handlers;
pub mod lifecycle;
pub mod obs;
pub mod ping;
pub mod priority;
pub mod swapper;
pub mod userscripts;
pub mod utils;
pub mod window;

use std::sync::{atomic::Ordering, mpsc};
use windows::{Win32::UI::WindowsAndMessaging::*, core::*};

pub fn run() {
    if obs::handle_cli_flags() {
        return;
    }

    lifecycle::register_instance();
    #[cfg(feature = "packaged")]
    {
        lifecycle::set_panic_hook().ok();
        lifecycle::installer_cleanup().ok();
    }

    if let Err(e) = app::init_fs() {
        eprintln!("failed to set all the files in place {}", e);
    }

    let window = app::create_main_window(None);
    let (_tx, rx) = mpsc::channel::<String>();
    #[cfg(feature = "auto-update")]
    {
        use utils::config;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        let main_thread_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
        if config("checkUpdates", true) {
            std::thread::spawn(move || {
                lifecycle::check_major_update();
                if let Some(new_js) = lifecycle::check_minor_update() {
                    _tx.send(new_js).ok();
                    unsafe {
                        PostThreadMessageW(main_thread_id, constants::WM_MINOR_UPDATE_READY, WPARAM(0), LPARAM(0)).unwrap();
                    }
                }
            });
        }
    }
    if utils::config("renderStats", false) {
        unsafe {
            SetTimer(None, 1, 100, None);
        }
    }
    let mut last_render_stats: Option<(u64, u64)> = None;
    let mut msg: MSG = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0).into() } {
        unsafe {
            _ = TranslateMessage(&msg);
        }

        if msg.message == constants::WM_MINOR_UPDATE_READY
            && let Ok(js_content) = rx.try_recv()
        {
            let script_id = crate::SCRIPT_ID.lock().unwrap();
            println!("updating js, {}", *script_id);

            let old_script_str = utils::create_utf_string(&*script_id);
            let new_script_str = utils::create_utf_string(js_content);

            unsafe {
                window.webview.RemoveScriptToExecuteOnDocumentCreated(PCWSTR(old_script_str.as_ptr())).ok();
                window.webview.AddScriptToExecuteOnDocumentCreated(PCWSTR(new_script_str.as_ptr()), None).ok();
            }
        }

        if msg.message == WM_TIMER {
            let ptr = app::SHARED_STATS_PTR.load(Ordering::SeqCst);
            if ptr != 0 {
                let shared = unsafe { &*(ptr as *const app::SharedStats) };
                let current = (shared.fps, shared.frame_ns);

                if shared.fps > 0 && last_render_stats != Some(current) {
                    last_render_stats = Some(current);

                    let payload = format!("{{\"fpsInfo\":{}}}", shared.fps);
                    let payload_str = utils::create_utf_string(payload);

                    unsafe {
                        window.webview.PostWebMessageAsJson(PCWSTR(payload_str.as_ptr())).ok();
                    }
                }
            }
        }

        unsafe {
            DispatchMessageW(&msg);
        }
    }

    crate::CONFIG.lock().unwrap().save();
}
