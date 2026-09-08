pub mod app;
pub mod bridge;
pub mod handlers;
pub mod renderer;
pub mod rpc;
pub mod window;

// mirrors platform::windows::utils::config
pub fn config<T: serde::de::DeserializeOwned>(setting: &str, default: T) -> T {
    crate::CONFIG.lock().unwrap().get(setting).unwrap_or(default)
}

pub fn run() {
    app::run();
}
