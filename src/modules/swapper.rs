use std::{collections::HashMap, fs};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::{
    Win32::{
        Foundation::*,
        System::Com::{StructuredStorage::*, *},
    },
    core::*,
};

use crate::shared::swapper;
use crate::utils;

pub fn load(window: &ICoreWebView2) -> HashMap<String, IStream> {
    let swap_dir = crate::shared::paths::settings_dir().join("swapper");
    fs::create_dir_all(&swap_dir).unwrap_or_default();

    let mut swaps = HashMap::new();
    for (relative_path, file_content) in swapper::load_all(&swap_dir) {
        unsafe {
            let url = (
                format!("*://krunker.io/{}*", relative_path),
                format!("*://*.krunker.io/{}*", relative_path),
            );

            for url_part in [&url.0, &url.1] {
                if let Err(e) =
                    window.AddWebResourceRequestedFilter(PCWSTR(utils::create_utf_string(url_part).as_ptr()), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)
                {
                    eprintln!("Failed to add web resource requested filter: {}", e);
                }
            }
        }

        unsafe {
            let Ok(stream) = CreateStreamOnHGlobal(HGLOBAL::default(), true) else {
                eprintln!("swapper: could not create stream for {}", relative_path);
                continue;
            };
            if stream.Write(file_content.as_ptr() as *const _, file_content.len() as u32, None).is_err() {
                eprintln!("swapper: could not write stream for {}", relative_path);
                continue;
            }
            stream.Seek(0, STREAM_SEEK_SET, None).ok();
            swaps.insert(relative_path, stream);
        }
    }
    swaps
}
