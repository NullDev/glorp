use cef::{rc::*, *};
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{LazyLock, Mutex},
    time,
};

use super::{bridge, devtools};

static LAST_CONNECTED_LOBBY: LazyLock<Mutex<IpAddr>> = LazyLock::new(|| Mutex::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

// replaces GetDevToolsProtocolEventReceiver which has no CEF equivalent
wrap_dev_tools_message_observer! {
    pub struct GlorpDevToolsObserver;

    impl DevToolsMessageObserver {
        fn on_dev_tools_event(&self, _browser: Option<&mut Browser>, method: Option<&CefString>, params: Option<&[u8]>) {
            let (Some(method), Some(params)) = (method, params) else { return };
            if method.to_string() != "Network.webSocketCreated" {
                return;
            }

            let Ok(json) = serde_json::from_slice::<serde_json::Value>(params) else { return };
            let Some(url) = json.get("url").and_then(|u| u.as_str()) else { return };
            if !url.contains("lobby-") {
                return;
            }

            let Some(host) = url.split("://").nth(1).and_then(|s| s.split('/').next()) else { return };
            let host = host.split(':').next().unwrap_or(host);

            if let Ok(ips) = dns_lookup::lookup_host(host)
                && let Some(ip) = ips.into_iter().next()
            {
                *LAST_CONNECTED_LOBBY.lock().unwrap() = ip;
            }
        }
    }
}

pub fn load(browser: &mut Browser) -> Option<Registration> {
    devtools::enable_network(browser);
    let host = browser.host()?;
    let mut observer = GlorpDevToolsObserver::new();
    host.add_dev_tools_message_observer(Some(&mut observer))
}

pub fn ping(frame: &mut Frame) {
    let addr = *LAST_CONNECTED_LOBBY.lock().unwrap();
    let result = ping_rs::send_ping(
        &addr,
        time::Duration::from_secs(1),
        Default::default(),
        Some(&ping_rs::PingOptions { ttl: 128, dont_fragment: true }),
    );
    match result {
        Ok(reply) => bridge::dispatch(frame, &format!("{{\"pingInfo\":{}}}", reply.rtt)),
        // raw ICMP needs CAP_NET_RAW or a permissive net.ipv4.ping_group_range
        Err(e) => eprintln!("ping failed ({:?}); realPing needs ICMP permission on linux", e),
    }
}
