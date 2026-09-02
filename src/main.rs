#![cfg_attr(feature = "packaged", windows_subsystem = "windows")]
use std::{
    env,
    sync::{LazyLock, Mutex},
};

mod config;
mod platform;
mod shared;

static LAUNCH_ARGS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(env::args().skip(1).collect()));
static CONFIG: LazyLock<Mutex<config::Config>> = LazyLock::new(|| Mutex::new(config::Config::load()));
static JS_VERSION: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new("0.0.0".to_string()));
static SCRIPT_ID: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

#[cfg(windows)]
fn main() {
    platform::backend::run();
}

#[cfg(target_os = "linux")]
fn main() {
    platform::backend::run();
}
