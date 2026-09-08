use cef::{
    rc::*,
    wrapper::message_router::MessageRouterBrowserSideHandlerCallbacks,
    wrapper::byte_read_handler::{ByteReadHandler, ByteStream},
    wrapper::stream_resource_handler::StreamResourceHandler,
    *,
};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use super::{bridge, config, window};
use crate::shared::{blocklist, paths, swapper, urls};

static BLOCKLIST: LazyLock<Vec<String>> = LazyLock::new(|| {
    if !config("blocklist", true) {
        return Vec::new();
    }
    let path = paths::settings_dir().join("user_blocklist.json");
    let user = std::fs::read_to_string(&path).ok().and_then(|s| blocklist::try_parse(&s).ok()).unwrap_or_default();
    blocklist::merge(blocklist::defaults(), &user)
});

static SWAPS: LazyLock<HashMap<String, Vec<u8>>> = LazyLock::new(|| {
    if !config("swapper", true) {
        return HashMap::new();
    }
    swapper::load_all(&paths::settings_dir().join("swapper")).into_iter().collect()
});

// mirrors the filename extraction in the Windows resource handler
fn swap_key(url: &str) -> Option<&str> {
    url.split("krunker.io/").nth(1).and_then(|s| s.split('?').next())
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "wasm" => "application/wasm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

fn request_url(request: Option<&mut Request>) -> Option<String> {
    let request = request?;
    let userfree = request.url();
    Some(CefStringUtf16::from(&userfree).to_string())
}

wrap_resource_request_handler! {
    pub struct GlorpResourceRequestHandler;

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let Some(url) = request_url(request) else { return ReturnValue::CONTINUE };
            if blocklist::is_blocked(&BLOCKLIST, &url) {
                return ReturnValue::CANCEL;
            }
            ReturnValue::CONTINUE
        }

        fn resource_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
        ) -> Option<ResourceHandler> {
            let url = request_url(request)?;
            if !url.contains("krunker.io") {
                return None;
            }
            let key = swap_key(&url)?;
            let bytes = SWAPS.get(key)?.clone();

            let mut read_handler = ByteReadHandler::new(Arc::new(Mutex::new(ByteStream::new(bytes))));
            let stream = stream_reader_create_for_handler(Some(&mut read_handler))?;
            Some(StreamResourceHandler::new_with_stream(mime_for(key).to_string(), stream))
        }
    }
}

wrap_request_handler! {
    pub struct GlorpRequestHandler;

    impl RequestHandler {
        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _is_navigation: ::std::os::raw::c_int,
            _is_download: ::std::os::raw::c_int,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut ::std::os::raw::c_int>,
        ) -> Option<ResourceRequestHandler> {
            Some(GlorpResourceRequestHandler::new())
        }

        fn on_before_browse(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _user_gesture: ::std::os::raw::c_int,
            _is_redirect: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int {
            bridge::router().on_before_browse(browser.map(|b| b.clone()), frame.map(|f| f.clone()));
            0
        }

        fn on_render_process_terminated(
            &self,
            browser: Option<&mut Browser>,
            _status: TerminationStatus,
            _error_code: ::std::os::raw::c_int,
            _error_string: Option<&CefString>,
        ) {
            bridge::router().on_render_process_terminated(browser.map(|b| b.clone()));
        }
    }
}

// CEF reports Windows virtual key codes on every platform
const VK_F4: i32 = 0x73;
const VK_F5: i32 = 0x74;
const VK_F6: i32 = 0x75;
const VK_F11: i32 = 0x7A;
const VK_F12: i32 = 0x7B;

fn frame_url(browser: &mut Browser) -> Option<String> {
    let frame = browser.main_frame()?;
    let userfree = frame.url();
    Some(CefStringUtf16::from(&userfree).to_string())
}

// mirrors handle_accelerator_key in the Windows backend
wrap_keyboard_handler! {
    pub struct GlorpKeyboardHandler;

    impl KeyboardHandler {
        fn on_key_event(
            &self,
            browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: Option<&mut sys::XEvent>,
        ) -> ::std::os::raw::c_int {
            let (Some(browser), Some(event)) = (browser, event) else { return 0 };
            if event.type_ != KeyEventType::RAWKEYDOWN {
                return 0;
            }

            match event.windows_key_code {
                VK_F4 | VK_F6 => {
                    // TODO: reset CPU throttle to 1.0, as Windows does
                    let current = frame_url(browser).unwrap_or_default();
                    let target = urls::new_lobby_url(&current);
                    if let Some(frame) = browser.main_frame() {
                        frame.load_url(Some(&CefString::from(target.as_str())));
                    }
                    1
                }
                VK_F5 => {
                    // TODO: reset CPU throttle to 1.0, as Windows does
                    browser.reload();
                    1
                }
                VK_F11 => {
                    window::toggle_fullscreen();
                    1
                }
                VK_F12 => {
                    if let Some(host) = browser.host() {
                        host.show_dev_tools(None, None, None, None);
                    }
                    1
                }
                _ => 0,
            }
        }
    }
}

// mirrors set_permission_requested_handler in the Windows backend
wrap_permission_handler! {
    struct GlorpPermissionHandler;

    impl PermissionHandler {
        fn on_show_permission_prompt(
            &self,
            _browser: Option<&mut Browser>,
            _prompt_id: u64,
            _requesting_origin: Option<&CefString>,
            _requested_permissions: u32,
            callback: Option<&mut PermissionPromptCallback>,
        ) -> ::std::os::raw::c_int {
            let Some(callback) = callback else { return 0 };
            callback.cont(PermissionRequestResult::ACCEPT);
            1
        }
    }
}

wrap_life_span_handler! {
    pub struct GlorpLifeSpanHandler;

    impl LifeSpanHandler {
        fn on_before_close(&self, browser: Option<&mut Browser>) {
            bridge::router().on_before_close(browser.map(|b| b.clone()));
        }
    }
}

wrap_client! {
    pub struct GlorpClient;

    impl Client {
        fn permission_handler(&self) -> Option<PermissionHandler> {
            Some(GlorpPermissionHandler::new())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(GlorpRequestHandler::new())
        }

        fn keyboard_handler(&self) -> Option<KeyboardHandler> {
            Some(GlorpKeyboardHandler::new())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(GlorpLifeSpanHandler::new())
        }

        fn on_process_message_received(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> ::std::os::raw::c_int {
            let handled = bridge::router().on_process_message_received(
                browser.map(|b| b.clone()),
                frame.map(|f| f.clone()),
                source_process,
                message.map(|m| m.clone()),
            );
            i32::from(handled)
        }
    }
}
