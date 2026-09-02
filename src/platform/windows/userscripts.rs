use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::core::*;

use crate::shared::userscripts;
use super::utils;

pub fn load(webview: &ICoreWebView2, social: bool) -> Result<()> {
    let scripts_dir = if social {
        crate::shared::paths::settings_dir().join("scripts").join("social")
    } else {
        crate::shared::paths::settings_dir().join("scripts")
    };

    for parsed in userscripts::load_all(&scripts_dir) {
        unsafe { webview.AddScriptToExecuteOnDocumentCreated(PCWSTR(utils::create_utf_string(parsed).as_ptr()), None)? }
    }

    Ok(())
}
