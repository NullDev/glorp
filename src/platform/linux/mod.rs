pub mod app;
pub mod bridge;
pub mod devtools;
pub mod ping;
pub mod handlers;
pub mod instance;
pub mod renderer;
pub mod rpc;
pub mod window;

// mirrors platform::windows::utils::config
pub fn config<T: serde::de::DeserializeOwned>(setting: &str, default: T) -> T {
    crate::CONFIG.lock().unwrap().get(setting).unwrap_or(default)
}

pub fn launch_args() -> Vec<String> {
    std::env::args().skip(1).filter(|a| !a.starts_with("--")).collect()
}

pub fn run() {
    app::run();
}
