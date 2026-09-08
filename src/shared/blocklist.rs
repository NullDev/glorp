use std::collections::HashSet;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct UserBlocklist {
    pub blocked: HashSet<String>,
    pub disabled_defaults: HashSet<String>,
}

pub fn merge(defaults: Vec<String>, user: &UserBlocklist) -> Vec<String> {
    defaults
        .into_iter()
        .filter(|url| !user.disabled_defaults.contains(url))
        .chain(user.blocked.iter().cloned())
        .collect()
}

#[allow(dead_code)] // used by the linux only
pub fn parse(contents: &str) -> UserBlocklist {
    try_parse(contents).unwrap_or_default()
}

pub fn try_parse(contents: &str) -> Result<UserBlocklist, serde_json::Error> {
    serde_json::from_str(contents)
}

pub fn defaults() -> Vec<String> {
    serde_json::from_str(crate::shared::constants::DEFAULT_BLOCKLIST).unwrap_or_default()
}

// WebView2 takes glob strings directly; CEF needs us to match them ourselves
#[allow(dead_code)] // used by the linux only
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut backtrack) = (usize::MAX, 0usize);

    while ti < t.len() {
        if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            backtrack = ti;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            backtrack += 1;
            ti = backtrack;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[allow(dead_code)] // used by the linux only
pub fn is_blocked(patterns: &[String], url: &str) -> bool {
    patterns.iter().any(|p| glob_match(p, url))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(blocked: &[&str], disabled: &[&str]) -> UserBlocklist {
        UserBlocklist {
            blocked: blocked.iter().map(|s| s.to_string()).collect(),
            disabled_defaults: disabled.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn keeps_defaults_when_nothing_disabled() {
        let out = merge(vec!["a".into(), "b".into()], &user(&[], &[]));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn drops_disabled_defaults() {
        let out = merge(vec!["a".into(), "b".into()], &user(&[], &["a"]));
        assert_eq!(out, vec!["b".to_string()]);
    }

    #[test]
    fn appends_user_entries() {
        let out = merge(vec!["a".into()], &user(&["z"], &[]));
        assert!(out.contains(&"z".to_string()));
        assert!(out.contains(&"a".to_string()));
    }

    #[test]
    fn malformed_json_yields_empty_user_list() {
        let parsed = parse("{ not json");
        assert!(parsed.blocked.is_empty());
        assert!(parsed.disabled_defaults.is_empty());
    }

    #[test]
    fn every_bundled_default_is_valid_json() {
        let d = defaults();
        assert!(!d.is_empty(), "DEFAULT_BLOCKLIST failed to parse");
        assert!(d.iter().all(|p| p.contains("://")), "every entry is a URL pattern");
    }

    #[test]
    fn subdomain_wildcard_matches_real_ad_url() {
        assert!(glob_match("*://*.doubleclick.net/*", "https://googleads.g.doubleclick.net/pagead/ads?x=1"));
    }

    #[test]
    fn scheme_wildcard_matches_both_schemes() {
        assert!(glob_match("*://*.youtube.com/*", "http://www.youtube.com/x"));
        assert!(glob_match("*://*.youtube.com/*", "https://www.youtube.com/x"));
    }

    #[test]
    fn trailing_star_matches_query_string() {
        assert!(glob_match("*://krunker.io/service-worker.js*", "https://krunker.io/service-worker.js?v=3"));
    }

    #[test]
    fn does_not_match_unrelated_host() {
        assert!(!glob_match("*://*.doubleclick.net/*", "https://krunker.io/js/game.js"));
    }

    // a bad matcher that blocks the game itself is worse than no blocklist
    #[test]
    fn core_game_assets_are_never_blocked() {
        let d = defaults();
        for url in [
            "https://krunker.io/",
            "https://krunker.io/js/game.js",
            "https://assets.krunker.io/models/weapon_0.obj",
            "wss://lobby-fra.krunker.io/socket",
        ] {
            assert!(!is_blocked(&d, url), "blocklist wrongly blocked {url}");
        }
    }

    #[test]
    fn known_trackers_are_blocked() {
        let d = defaults();
        for url in [
            "https://googleads.g.doubleclick.net/pagead/ads?x=1",
            "https://platform.twitter.com/widgets/widget_iframe.html",
            "https://www.google-analytics.com/analytics.js",
            "https://krunker.io/service-worker.js",
        ] {
            assert!(is_blocked(&d, url), "blocklist missed {url}");
        }
    }

    // don't fail on malformed user file
    #[test]
    fn malformed_user_file_still_yields_the_defaults() {
        let out = merge(defaults(), &parse("{ not json"));
        assert_eq!(out.len(), defaults().len());
    }
}
