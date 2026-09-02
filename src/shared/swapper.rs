use std::fs;
use std::path::{Component, Path};

pub type Swap = (String, Vec<u8>);

// works on both
fn relative_url_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            // dont guess
            _ => return None,
        }
    }
    if parts.is_empty() { None } else { Some(parts.join("/")) }
}

fn recurse(root: &Path, dir: &Path, out: &mut Vec<Swap>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            recurse(root, &path, out);
        } else if file_type.is_file() {
            let Some(relative) = relative_url_path(root, &path) else {
                continue;
            };
            let Ok(bytes) = fs::read(&path) else {
                eprintln!("swapper: skipping unreadable file {}", path.display());
                continue;
            };
            out.push((relative, bytes));
        }
    }
}

pub fn load_all(root: &Path) -> Vec<Swap> {
    let mut out = Vec::new();
    recurse(root, root, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("glorp-swapper-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_files_recursively_with_url_separators() {
        let root = tmpdir("recursive");
        fs::create_dir_all(root.join("img").join("sub")).unwrap();
        fs::write(root.join("top.js"), b"a").unwrap();
        fs::write(root.join("img").join("sub").join("deep.png"), b"bb").unwrap();

        let mut got: Vec<String> = load_all(&root).into_iter().map(|(p, _)| p).collect();
        got.sort();
        assert_eq!(got, vec!["img/sub/deep.png".to_string(), "top.js".to_string()]);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn returns_file_contents() {
        let root = tmpdir("contents");
        fs::write(root.join("a.txt"), b"hello").unwrap();

        let swaps = load_all(&root);
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].1, b"hello");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_directory_yields_empty_not_panic() {
        let missing = std::env::temp_dir().join("glorp-swapper-does-not-exist");
        assert!(load_all(&missing).is_empty());
    }

    #[test]
    fn empty_directory_yields_empty() {
        let root = tmpdir("empty");
        assert!(load_all(&root).is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn relative_path_uses_forward_slashes() {
        let root = Path::new("/root/swapper");
        let file = Path::new("/root/swapper/img/x.png");
        assert_eq!(relative_url_path(root, file).as_deref(), Some("img/x.png"));
    }
}
