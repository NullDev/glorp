use cef::{rc::*, *};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
use std::thread;

// replaces CreateMutexW + WM_COPYDATA.
const SOCKET_NAME: &[u8] = b"glorp-7e0f405e-fe65-493a-acf0-9719b85697cd";

thread_local! {
    static BROWSER: RefCell<Option<Browser>> = const { RefCell::new(None) };
}

pub fn set_browser(browser: Browser) {
    BROWSER.set(Some(browser));
}

wrap_task! {
    pub struct ParseArgsTask {
        args: String,
    }

    impl Task {
        fn execute(&self) {
            BROWSER.with_borrow(|browser| {
                let Some(browser) = browser else { return };
                let Some(frame) = browser.main_frame() else { return };
                let quoted = serde_json::to_string(&self.args).unwrap_or_else(|_| "\"\"".to_string());
                let js = format!("window.glorp && window.glorp.parseArgs && window.glorp.parseArgs({quoted});");
                frame.execute_java_script(Some(&CefString::from(js.as_str())), None, 0);
            });
        }
    }
}

fn address() -> Option<SocketAddr> {
    SocketAddr::from_abstract_name(SOCKET_NAME).ok()
}

pub fn acquire() -> bool {
    let Some(addr) = address() else {
        return true;
    };

    if let Ok(mut stream) = UnixStream::connect_addr(&addr) {
        let args = super::launch_args().join(" ");
        if !args.is_empty() {
            stream.write_all(args.as_bytes()).ok();
        }
        return false;
    }

    let Ok(listener) = UnixListener::bind_addr(&addr) else {
        return true;
    };

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut args = String::new();
            if stream.read_to_string(&mut args).is_err() || args.is_empty() {
                continue;
            }
            // CefFrame is UI-thread only
            let mut task = ParseArgsTask::new(args);
            post_task(ThreadId::UI, Some(&mut task));
        }
    });

    true
}
