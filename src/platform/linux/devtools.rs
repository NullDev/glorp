use cef::*;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

// CDP message ids only need to be unique per browser
static NEXT_ID: AtomicI32 = AtomicI32::new(1);

fn next_id() -> i32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn call(browser: &mut Browser, method: &str, params: Option<DictionaryValue>) {
    let Some(host) = browser.host() else { return };
    let mut params = params;
    host.execute_dev_tools_method(next_id(), Some(&CefString::from(method)), params.as_mut());
}

// format!() form used by utils::set_cpu_throttling on Windows does not port
fn dict() -> Option<DictionaryValue> {
    dictionary_value_create()
}

// mirrors LAST_THROTTLE_BITS in the Windows utils
static LAST_THROTTLE_BITS: AtomicU32 = AtomicU32::new(1.0f32.to_bits());

pub fn set_cpu_throttling(browser: &mut Browser, rate: f32) {
    if LAST_THROTTLE_BITS.swap(rate.to_bits(), Ordering::Relaxed) == rate.to_bits() {
        return;
    }
    let Some(params) = dict() else { return };
    params.set_double(Some(&CefString::from("rate")), rate as f64);
    call(browser, "Emulation.setCPUThrottlingRate", Some(params));
}

pub fn reset_throttle_cache() {
    LAST_THROTTLE_BITS.store(1.0f32.to_bits(), Ordering::Relaxed);
}

pub fn clear_cache(browser: &mut Browser) {
    set_cpu_throttling(browser, 1.0);

    call(browser, "Network.clearBrowserCache", None);

    if let Some(params) = dict() {
        params.set_string(Some(&CefString::from("origin")), Some(&CefString::from("*")));
        params.set_string(Some(&CefString::from("storageTypes")), Some(&CefString::from("all")));
        call(browser, "Storage.clearDataForOrigin", Some(params));
    }

    browser.reload();
}

pub fn enable_network(browser: &mut Browser) {
    call(browser, "Network.enable", None);
}
