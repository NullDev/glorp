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

    // don't fail on malformed user file
    #[test]
    fn malformed_user_file_still_yields_the_defaults() {
        let out = merge(defaults(), &parse("{ not json"));
        assert_eq!(out.len(), defaults().len());
    }
}
