use cef::{rc::*, *};

use super::config;
use crate::shared::window_state::WindowState;

const DEFAULT_WIDTH: i32 = 1600;
const DEFAULT_HEIGHT: i32 = 900;

// cant create our own HWND and embedding
wrap_window_delegate! {
    struct GlorpWindowDelegate {
        browser_view: BrowserView,
        start_mode: String,
        state: Option<WindowState>,
    }

    impl ViewDelegate {}

    impl PanelDelegate {}

    impl WindowDelegate {
        fn initial_bounds(&self, _window: Option<&mut Window>) -> Rect {
            match self.state {
                Some(state) if state.position.width() > 0 && state.position.height() > 0 => Rect {
                    x: state.position.left,
                    y: state.position.top,
                    width: state.position.width(),
                    height: state.position.height(),
                },
                _ => Rect { x: 0, y: 0, width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT },
            }
        }

        fn initial_show_state(&self, _window: Option<&mut Window>) -> ShowState {
            match self.start_mode.as_str() {
                "Maximized" => ShowState::MAXIMIZED,
                "Borderless Fullscreen" => ShowState::FULLSCREEN,
                _ => ShowState::NORMAL,
            }
        }

        fn is_frameless(&self, _window: Option<&mut Window>) -> ::std::os::raw::c_int {
            i32::from(self.start_mode == "Borderless Fullscreen")
        }

        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            let mut view: View = (&self.browser_view).into();
            window.add_child_view(Some(&mut view));
            window.show();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            quit_message_loop();
        }
    }
}

pub fn create(client: &mut Client, url: &str) {
    let start_mode = config("startMode", "Remember Previous".to_string());
    let state = if start_mode == "Remember Previous" || start_mode == "Custom" {
        config("lastPosition", None::<WindowState>)
    } else {
        None
    };

    let url = CefString::from(url);
    let Some(browser_view) = browser_view_create(Some(client), Some(&url), Some(&BrowserSettings::default()), None, None, None) else {
        eprintln!("browser_view_create failed");
        return;
    };

    let mut delegate = GlorpWindowDelegate::new(browser_view, start_mode, state);
    if window_create_top_level(Some(&mut delegate)).is_none() {
        eprintln!("window_create_top_level failed");
    }
}
