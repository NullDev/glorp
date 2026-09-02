#[derive(Copy, Clone, serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct Position {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct WindowState {
    pub fullscreen: bool,
    pub position: Position,
}

// used by Linux views-backend for initial_bounds
#[allow(dead_code)]
impl Position {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}
