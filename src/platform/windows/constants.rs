use windows::Win32::UI::WindowsAndMessaging::WM_USER;

pub const INSTANCE_MUTEX: &str = "Global\\7e0f405e-fe65-493a-acf0-9719b85697cd";
pub const WM_MINOR_UPDATE_READY: u32 = WM_USER + 5;
