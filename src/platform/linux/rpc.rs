use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::sync::{Mutex, OnceLock};

use super::config;
use crate::shared::constants;

static CLIENT: OnceLock<Mutex<Option<DiscordIpcClient>>> = OnceLock::new();

// discord-rich-presence talks over a unix socket here, so this needs no porting
pub fn init() {
    let client = if config("discordRPC", true) {
        let mut client = DiscordIpcClient::new(constants::DISCORD_CLIENT_ID);
        client.connect().ok();
        Some(client)
    } else {
        None
    };
    let _ = CLIENT.set(Mutex::new(client));
}

pub fn update(mode: &str, map: &str) {
    let Some(cell) = CLIENT.get() else { return };
    let Ok(mut guard) = cell.lock() else { return };
    let Some(client) = guard.as_mut() else { return };

    let state = format!("{} on {}", mode, map);
    let activity = activity::Activity::new().details("Krunker").state(&state).assets(activity::Assets::new());
    if let Err(e) = client.set_activity(activity) {
        eprintln!("Failed to set rpc activity: {}", e);
    }
}
