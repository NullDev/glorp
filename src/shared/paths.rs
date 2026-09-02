use std::path::PathBuf;

#[cfg(windows)]
pub fn settings_dir() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap()).join("Documents").join("glorp")
}

#[cfg(unix)]
pub fn settings_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("glorp");
    }
    PathBuf::from(std::env::var("HOME").expect("HOME is not set")).join(".config").join("glorp")
}

pub fn resources_dir() -> PathBuf {
    std::env::current_exe()
        .expect("cannot resolve current exe")
        .parent()
        .expect("exe has no parent directory")
        .join("resources")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_dir_is_absolute_and_ends_with_glorp() {
        let dir = settings_dir();
        assert!(dir.is_absolute(), "settings_dir must be absolute, got {dir:?}");
        assert_eq!(dir.file_name().unwrap(), "glorp");
    }

    #[test]
    fn resources_dir_is_absolute() {
        assert!(resources_dir().is_absolute());
    }

    #[cfg(windows)]
    #[test]
    fn windows_layout_is_documents_glorp() {
        let dir = settings_dir();
        let s = dir.to_string_lossy().replace('/', "\\");
        assert!(s.ends_with("\\Documents\\glorp"), "unexpected Windows layout: {s}");
    }

    #[cfg(unix)]
    #[test]
    fn unix_prefers_xdg_config_home() {
        let dir = settings_dir();
        let s = dir.to_string_lossy();
        assert!(
            s.contains("/.config/glorp") || std::env::var("XDG_CONFIG_HOME").is_ok(),
            "expected XDG or ~/.config fallback, got {s}"
        );
    }
}
