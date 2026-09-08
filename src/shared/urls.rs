// F6/F4: rejoin the pool excluding the lobby we're in
pub fn new_lobby_url(current_url: &str) -> String {
    current_url
        .split_once("game=")
        .map(|(_before, after)| after.trim())
        .filter(|id| !id.is_empty())
        .map(|id| format!("https://krunker.io/?exclude={}", id))
        .unwrap_or_else(|| "https://krunker.io/".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_the_current_lobby() {
        assert_eq!(new_lobby_url("https://krunker.io/?game=FRA:abc123"), "https://krunker.io/?exclude=FRA:abc123");
    }

    #[test]
    fn plain_url_when_not_in_a_game() {
        assert_eq!(new_lobby_url("https://krunker.io/"), "https://krunker.io/");
    }

    #[test]
    fn plain_url_when_game_id_is_empty() {
        assert_eq!(new_lobby_url("https://krunker.io/?game="), "https://krunker.io/");
    }

    #[test]
    fn handles_an_already_excluded_url() {
        assert_eq!(new_lobby_url("https://krunker.io/?exclude=FRA:old"), "https://krunker.io/");
    }
}
