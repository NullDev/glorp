use cef::{wrapper::message_router::*, *};
use std::sync::{Arc, Mutex, OnceLock};

use super::{config, devtools, ping};
use crate::shared::{paths, userscripts};

// The frontend only ever uses postMessage / addEventListener / removeEventListener,
// so shimming those three onto cefQuery keeps bundle.js identical on both platforms
const WEBVIEW_SHIM: &str = r#"
window.__glorpErrors = [];
window.addEventListener("error", (e) => {
  window.__glorpErrors.push("ERR " + e.message + " @ " + (e.filename || "?") + ":" + e.lineno);
});
window.addEventListener("unhandledrejection", (e) => {
  window.__glorpErrors.push("REJECT " + String(e.reason && e.reason.stack ? e.reason.stack : e.reason));
});
(() => {
  if (window.chrome && window.chrome.webview) return;
  const listeners = new Set();
  window.chrome = window.chrome || {};
  window.chrome.webview = {
    postMessage(msg) {
      window.cefQuery({ request: String(msg), persistent: false, onSuccess() {}, onFailure() {} });
    },
    addEventListener(type, fn) { if (type === "message") listeners.add(fn); },
    removeEventListener(type, fn) { if (type === "message") listeners.delete(fn); },
  };
  // rust -> page, mirroring WebView2's event.data semantics
  window.__glorpDispatch = (data) => {
    for (const fn of listeners) {
      try { fn({ data }); } catch (e) { console.error(e); }
    }
  };
})();
"#;

static ROUTER: OnceLock<Arc<BrowserSideRouter>> = OnceLock::new();

pub fn router() -> &'static Arc<BrowserSideRouter> {
    ROUTER.get_or_init(|| {
        let router = BrowserSideRouter::new(MessageRouterConfig::default());
        router.add_handler(Arc::new(GlorpQueryHandler), false);
        router
    })
}

fn read_bundle() -> String {
    let path = paths::resources_dir().join("bundle.js");
    match std::fs::read_to_string(&path) {
        Ok(js) => js,
        Err(_) => {
            #[cfg(feature = "editor-ignore")]
            {
                return include_str!("../../../target/bundle.js").to_string();
            }
            #[allow(unreachable_code)]
            {
                eprintln!("bridge: no bundle.js at {} - client UI will be absent", path.display());
                String::new()
            }
        }
    }
}

pub fn document_start_script(social: bool) -> String {
    let mut out = String::from(WEBVIEW_SHIM);
    out.push_str(&read_bundle());

    if config("userscripts", true) {
        let dir = if social {
            paths::settings_dir().join("scripts").join("social")
        } else {
            paths::settings_dir().join("scripts")
        };
        for script in userscripts::load_all(&dir) {
            out.push('\n');
            out.push_str(&script);
        }
    }
    out
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

// `payload` must already be a JS literal
pub fn dispatch(frame: &mut Frame, payload: &str) {
    let js = format!("window.__glorpDispatch && window.__glorpDispatch({payload});");
    frame.execute_java_script(Some(&CefString::from(js.as_str())), None, 0);
}

pub fn dispatch_str(frame: &mut Frame, message: &str) {
    dispatch(frame, &js_string(message));
}

fn send_info(frame: &mut Frame) {
    let mut info = serde_json::Map::new();

    let mut settings = serde_json::json!(&*crate::CONFIG.lock().unwrap());

    if let Some(data) = settings.get_mut("data").and_then(|d| d.as_object_mut()) {
        data.insert("rawInput".to_string(), serde_json::Value::Bool(false));
    }
    info.insert("settings".to_string(), settings);
    info.insert("version".to_string(), serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string()));

    let launch_args = crate::LAUNCH_ARGS.lock().unwrap();
    if !launch_args.is_empty() {
        info.insert("launchArgs".to_string(), serde_json::Value::String(launch_args.join(" ")));
    }
    drop(launch_args);

    let json = serde_json::to_string(&info).unwrap_or_else(|_| "{}".to_string());
    dispatch(frame, &json);
}

// xdg-open, replacing explorer.exe
fn open_settings_subpath(target: &str) {
    let path = match target {
        "blocklist" => paths::settings_dir().join("user_blocklist.json"),
        "swapper" => paths::settings_dir().join("swapper"),
        "userscripts" => paths::settings_dir().join("scripts"),
        _ => return,
    };
    std::process::Command::new("xdg-open").arg(path).spawn().ok();
}

// mirrors parse_web_message_value in the Windows backend
fn parse_value(value: &str) -> serde_json::Value {
    if let Ok(b) = value.parse::<bool>() {
        serde_json::Value::Bool(b)
    } else if let Ok(i) = value.parse::<i64>() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = value.parse::<f64>() {
        serde_json::Number::from_f64((f * 100.0).round() / 100.0)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(value.to_string()))
    } else {
        serde_json::Value::String(value.to_string())
    }
}

struct GlorpQueryHandler;

impl BrowserSideHandler for GlorpQueryHandler {
    fn on_query_str(
        &self,
        browser: Option<Browser>,
        frame: Option<Frame>,
        _query_id: i64,
        request: &str,
        _persistent: bool,
        callback: Arc<Mutex<dyn BrowserSideCallback>>,
    ) -> bool {
        let parts: Vec<&str> = request.split(", ").map(|s| s.trim()).collect();
        let (mut browser, mut frame) = (browser, frame);

        match parts.as_slice() {
            ["set-config", setting, value] => {
                crate::CONFIG.lock().unwrap().set(setting, parse_value(value));
            }
            ["get-info"] => {
                if let Some(frame) = frame.as_mut() {
                    send_info(frame);
                }
            }
            ["close"] => {
                quit_message_loop();
            }
            ["open", target] => open_settings_subpath(target),
            ["rpc-update", mode, map] => super::rpc::update(mode, map),

            // no Linux equivalent: these drive webview-dll on Windows
            ["drag", _] | ["toggle-rboost", _] | ["obs-plugin", _] => {}

            ["throttle", status] => {
                let setting = if *status == "game" { "throttle" } else { "inMenuThrottle" };
                if let Some(browser) = browser.as_mut() {
                    devtools::set_cpu_throttling(browser, config(setting, 1.0));
                }
            }
            ["clear-cache"] => {
                if let Some(browser) = browser.as_mut() {
                    devtools::clear_cache(browser);
                }
            }
            ["ping"] => {
                if let Some(frame) = frame.as_mut() {
                    ping::ping(frame);
                }
            }

            _ => {}
        }

        callback.lock().unwrap().success_str("");
        true
    }
}
