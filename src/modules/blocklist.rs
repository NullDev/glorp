use std::{fs, io::Write};

use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::core::*;

use crate::shared::blocklist;
use crate::utils;

const EXAMPLE_BLOCKLIST: &str = r#"
{
    "blocked": [
        "*://example1.com",
        "*://*.example2.com/*"
    ],
    "disabled_defaults": [
        ""
    ]
}"#;

fn add_filters(webview_window: &ICoreWebView2, urls: Vec<String>) {
    for url in urls {
        unsafe {
            let _ = webview_window.AddWebResourceRequestedFilter(
                PCWSTR(utils::create_utf_string(url).as_ptr()),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            );
        };
    }
}

pub fn load(webview_window: &ICoreWebView2) {
    let defaults = blocklist::defaults();
    let blocklist_path = crate::shared::paths::settings_dir().join("user_blocklist.json");

    let mut blocklist_file = if let Ok(file) = fs::OpenOptions::new().write(true).read(true).create(true).truncate(false).open(&blocklist_path) {
        file
    } else {
        eprintln!("can't open blocklist file");
        add_filters(webview_window, defaults);
        return;
    };

    if blocklist_file.metadata().unwrap().len() == 0 {
        blocklist_file.write_all(EXAMPLE_BLOCKLIST.as_bytes()).ok();
    }

    let blocklist_string = if let Ok(blocklist_string) = fs::read_to_string(&blocklist_path) {
        blocklist_string
    } else {
        eprintln!("can't read user blocklist file");
        blocklist_file.set_len(0).ok();
        blocklist_file.write_all(EXAMPLE_BLOCKLIST.as_bytes()).ok();
        add_filters(webview_window, defaults);
        return;
    };

    let user = match blocklist::try_parse(&blocklist_string) {
        Ok(config) => config,
        Err(_) => {
            eprintln!("can't parse user blocklist file");
            blocklist_file.set_len(0).ok();
            blocklist_file.write_all(EXAMPLE_BLOCKLIST.as_bytes()).ok();
            add_filters(webview_window, defaults);
            return;
        }
    };

    add_filters(webview_window, blocklist::merge(defaults, &user));
}
