use cef::{args::Args, rc::*, *};
use std::{fs, io, sync::Mutex};

use super::{config, handlers, window};
use crate::shared::{constants, flaglist, paths};

const KRUNKER_URL: &str = "https://krunker.io";

static FLAGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn init_fs() -> io::Result<()> {
    let client_dir = paths::settings_dir();
    fs::create_dir_all(client_dir.join("swapper"))?;
    fs::create_dir_all(client_dir.join("scripts").join("social"))?;
    fs::create_dir_all(paths::resources_dir())?;

    let flaglist_path = client_dir.join("user_flags.json");
    let blocklist_path = client_dir.join("user_blocklist.json");
    if !flaglist_path.exists() {
        fs::write(&flaglist_path, constants::DEFAULT_FLAGS)?;
    }
    if !blocklist_path.exists() {
        fs::write(&blocklist_path, constants::DEFAULT_BLOCKLIST)?;
    }
    Ok(())
}

fn load_flags() -> Vec<String> {
    let path = paths::settings_dir().join("user_flags.json");
    let user = fs::read_to_string(&path).ok().and_then(|s| flaglist::try_parse(&s).ok()).unwrap_or_default();

    let mut flags = flaglist::filter_for_platform(flaglist::merge(flaglist::defaults(), &user));

    // matches create_main_window on Windows
    if config("uncapFps", true) {
        flags.push("--disable-frame-rate-limit".to_string());
    }
    flags
}

fn merge_list_switch(cmd: &mut CommandLine, key: &str, value: &str) {
    let name = CefString::from(key);
    let existing = if cmd.has_switch(Some(&name)) != 0 {
        let userfree = cmd.switch_value(Some(&name));
        CefStringUtf16::from(&userfree).to_string()
    } else {
        String::new()
    };
    let mut parts: Vec<&str> = existing.split(',').filter(|s| !s.is_empty()).collect();
    for v in value.split(',').filter(|s| !s.is_empty()) {
        if !parts.contains(&v) {
            parts.push(v);
        }
    }
    let joined = parts.join(",");
    cmd.append_switch_with_value(Some(&name), Some(&CefString::from(joined.as_str())));
}

wrap_browser_process_handler! {
    struct GlorpBrowserProcessHandler;

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let mut client = handlers::GlorpClient::new();
            window::create(&mut client, KRUNKER_URL);
        }
    }
}

wrap_app! {
    struct GlorpApp;

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(GlorpBrowserProcessHandler::new())
        }

        fn on_before_command_line_processing(&self, process_type: Option<&CefString>, command_line: Option<&mut CommandLine>) {
            if !process_type.map(|t| t.to_string().is_empty()).unwrap_or(true) {
                return;
            }
            let Some(cmd) = command_line else { return };

            for flag in FLAGS.lock().unwrap().iter() {
                let flag = flag.trim_start_matches("--");
                match flag.split_once('=') {
                    Some((k, v)) if k == "disable-features" || k == "enable-features" => merge_list_switch(cmd, k, v),
                    Some((k, v)) => cmd.append_switch_with_value(Some(&CefString::from(k)), Some(&CefString::from(v))),
                    None => cmd.append_switch(Some(&CefString::from(flag))),
                }
            }
        }
    }
}

pub fn run() {
    // has to precede execute_process cuz CEF 151 uses a versioned C ABI and rejects every struct without this handshake
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = Args::new();
    let mut app = GlorpApp::new();

    let code = execute_process(Some(args.as_main_args()), Some(&mut app), std::ptr::null_mut());
    if code >= 0 {
        std::process::exit(code);
    }

    if let Err(e) = init_fs() {
        eprintln!("failed to set all the files in place {}", e);
    }
    *FLAGS.lock().unwrap() = load_flags();

    let settings = Settings {
        user_agent: CefString::from("Electron"),
        locale: CefString::from("en-US"),
        root_cache_path: CefString::from(paths::settings_dir().join("browser").to_string_lossy().as_ref()),
        ..Default::default()
    };

    if initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut()) != 1 {
        eprintln!("cef initialize failed");
        std::process::exit(1);
    }

    run_message_loop();
    shutdown();

    crate::CONFIG.lock().unwrap().save();
}
