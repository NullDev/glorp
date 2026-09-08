use cef::{rc::*, wrapper::message_router::*, *};
use std::sync::{Arc, OnceLock};

use super::bridge;

static ROUTER: OnceLock<Arc<RendererSideRouter>> = OnceLock::new();

fn router() -> &'static Arc<RendererSideRouter> {
    ROUTER.get_or_init(|| RendererSideRouter::new(MessageRouterConfig::default()))
}

static SCRIPT: OnceLock<String> = OnceLock::new();
static SOCIAL_SCRIPT: OnceLock<String> = OnceLock::new();

fn frame_url(frame: &mut Frame) -> String {
    let userfree = frame.url();
    CefStringUtf16::from(&userfree).to_string()
}

wrap_render_process_handler! {
    pub struct GlorpRenderProcessHandler;

    impl RenderProcessHandler {
        fn on_context_created(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, context: Option<&mut V8Context>) {
            router().on_context_created(browser.map(|b| b.clone()), frame.as_ref().map(|f| (**f).clone()), context.map(|c| c.clone()));

            let Some(frame) = frame else { return };

            if frame.is_main() == 0 {
                return;
            }

            let social = frame_url(frame).contains("social.html");
            let cache = if social { &SOCIAL_SCRIPT } else { &SCRIPT };
            let script = cache.get_or_init(|| bridge::document_start_script(social));
            if script.is_empty() {
                return;
            }
            frame.execute_java_script(Some(&CefString::from(script.as_str())), None, 0);
        }

        fn on_context_released(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, context: Option<&mut V8Context>) {
            router().on_context_released(browser.map(|b| b.clone()), frame.map(|f| f.clone()), context.map(|c| c.clone()));
        }

        fn on_process_message_received(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> ::std::os::raw::c_int {
            let handled = router().on_process_message_received(
                browser.map(|b| b.clone()),
                frame.map(|f| f.clone()),
                Some(source_process),
                message.map(|m| m.clone()),
            );
            i32::from(handled)
        }
    }
}
