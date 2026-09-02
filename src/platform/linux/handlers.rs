use cef::{rc::*, *};

// mirrors set_permission_requested_handler in the Windows backend
wrap_permission_handler! {
    struct GlorpPermissionHandler;

    impl PermissionHandler {
        fn on_show_permission_prompt(
            &self,
            _browser: Option<&mut Browser>,
            _prompt_id: u64,
            _requesting_origin: Option<&CefString>,
            _requested_permissions: u32,
            callback: Option<&mut PermissionPromptCallback>,
        ) -> ::std::os::raw::c_int {
            let Some(callback) = callback else { return 0 };
            callback.cont(PermissionRequestResult::ACCEPT);
            1
        }
    }
}

wrap_client! {
    pub struct GlorpClient;

    impl Client {
        fn permission_handler(&self) -> Option<PermissionHandler> {
            Some(GlorpPermissionHandler::new())
        }
    }
}
