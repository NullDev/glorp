use std::collections::HashSet;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct UserFlags {
    pub flags: HashSet<String>,
    pub disabled_defaults: HashSet<String>,
}

// harmul switches under CEF
#[cfg(unix)]
pub const WINDOWS_ONLY_FLAGS: &[&str] = &["--enable-waitable-swap-chain", "--raise-timer-frequency"];

// no-ops under CEF
#[cfg(unix)]
pub const WINDOWS_ONLY_FEATURES: &[&str] = &[
    "msWebOOUI",
    "msPdfOOUI",
    "msSmartScreenProtection",
    "CalculateNativeWinOcclusion",
    "slow-dc-timer-interrupts-win",
];

#[allow(dead_code)] // only used by linux
pub fn parse(contents: &str) -> UserFlags {
    try_parse(contents).unwrap_or_default()
}

pub fn try_parse(contents: &str) -> Result<UserFlags, serde_json::Error> {
    serde_json::from_str(contents)
}

pub fn merge(defaults: Vec<String>, user: &UserFlags) -> Vec<String> {
    defaults
        .into_iter()
        .filter(|flag| !user.disabled_defaults.contains(flag))
        .chain(user.flags.iter().cloned())
        .collect()
}

pub fn defaults() -> Vec<String> {
    serde_json::from_str(crate::shared::constants::DEFAULT_FLAGS).unwrap_or_default()
}

// drop unused platform-specific switches
#[cfg(windows)]
pub fn filter_for_platform(flags: Vec<String>) -> Vec<String> {
    flags
}

#[cfg(unix)]
pub fn filter_for_platform(flags: Vec<String>) -> Vec<String> {
    flags
        .into_iter()
        .filter(|flag| !WINDOWS_ONLY_FLAGS.iter().any(|w| flag.starts_with(w)))
        .map(|flag| strip_windows_features(&flag))
        .filter(|flag| !flag.ends_with('='))
        .collect()
}

#[cfg(unix)]
fn strip_windows_features(flag: &str) -> String {
    let Some((prefix, list)) = flag.split_once('=') else {
        return flag.to_string();
    };
    if prefix != "--enable-features" && prefix != "--disable-features" {
        return flag.to_string();
    }
    let kept: Vec<&str> = list.split(',').filter(|feature| !WINDOWS_ONLY_FEATURES.contains(feature)).collect();
    format!("{prefix}={}", kept.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(flags: &[&str], disabled: &[&str]) -> UserFlags {
        UserFlags {
            flags: flags.iter().map(|s| s.to_string()).collect(),
            disabled_defaults: disabled.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn drops_disabled_defaults_and_appends_user_flags() {
        let out = merge(vec!["--a".into(), "--b".into()], &user(&["--z"], &["--a"]));
        assert!(!out.contains(&"--a".to_string()));
        assert!(out.contains(&"--b".to_string()));
        assert!(out.contains(&"--z".to_string()));
    }

    #[test]
    fn malformed_json_yields_empty_user_flags() {
        let parsed = parse("{ not json");
        assert!(parsed.flags.is_empty());
        assert!(parsed.disabled_defaults.is_empty());
    }

    #[test]
    fn bundled_defaults_parse() {
        assert!(defaults().len() > 20, "expected the full default flag set");
    }

    #[cfg(windows)]
    #[test]
    fn windows_filter_is_the_identity() {
        let input = vec![
            "--raise-timer-frequency".to_string(),
            "--disable-features=msWebOOUI,MediaRouter".to_string(),
        ];
        assert_eq!(filter_for_platform(input.clone()), input);
    }

    #[cfg(windows)]
    #[test]
    fn windows_defaults_pass_through_untouched() {
        let d = defaults();
        assert_eq!(filter_for_platform(d.clone()), d);
    }

    #[cfg(unix)]
    #[test]
    fn drops_windows_only_switches_on_linux() {
        let out = filter_for_platform(vec!["--raise-timer-frequency".to_string(), "--enable-quic".to_string()]);
        assert_eq!(out, vec!["--enable-quic".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn strips_edge_only_features_but_keeps_the_rest() {
        let out = filter_for_platform(vec!["--disable-features=msWebOOUI,MediaRouter".to_string()]);
        assert_eq!(out, vec!["--disable-features=MediaRouter".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn drops_a_feature_switch_that_becomes_empty() {
        let out = filter_for_platform(vec!["--disable-features=msWebOOUI".to_string()]);
        assert!(out.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn linux_keeps_the_performance_switches() {
        let out = filter_for_platform(defaults());
        for expected in ["--enable-gpu-rasterization", "--enable-zero-copy", "--disable-gpu-watchdog"] {
            assert!(out.iter().any(|f| f == expected), "lost {expected}");
        }
        assert!(!out.iter().any(|f| f.starts_with("--enable-waitable-swap-chain")));
        assert!(!out.iter().any(|f| f.contains("msWebOOUI")));
    }
}
