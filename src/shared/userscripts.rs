use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static METADATA_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)\A\s*\/\/ ==UserScript==.*?\/\/ ==\/UserScript=="#).unwrap());
static IIFE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)^\s*(?:['\"]use strict['\"];?\s*)?\(.*\)\s*\(\s*\)\s*;?\s*$"#).unwrap());

// TODO: everything
fn parse_metadata(content: &mut String) {
    if let Some(metadata_block) = METADATA_REGEX.find(content) {
        let metadata = metadata_block.as_str();
        if !metadata.contains("// @run-at document-start") {
            *content = format!("document.addEventListener('DOMContentLoaded', function() {{\n{}\n}});", content);
        }
    }
}

pub fn parse(mut content: String) -> String {
    if METADATA_REGEX.is_match(&content) {
        parse_metadata(&mut content);
    }

    // wrap it in an IIFE if it's not already
    if IIFE_REGEX.is_match(content.as_str()) {
        return content;
    }

    format!("(function() {{\n{}\n}})();", content)
}

pub fn load_all(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut scripts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("js")) {
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(content) => scripts.push(parse(content)),
            Err(e) => eprintln!("userscripts: skipping {}: {}", path.display(), e),
        }
    }
    scripts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_bare_script_in_iife() {
        let out = parse("console.log(1)".to_string());
        assert!(out.starts_with("(function()"));
        assert!(out.trim_end().ends_with("})();"));
    }

    #[test]
    fn leaves_existing_iife_alone() {
        let src = "(function(){ console.log(1) })();".to_string();
        assert_eq!(parse(src.clone()), src);
    }

    #[test]
    fn defers_script_without_run_at_document_start() {
        let src = "// ==UserScript==\n// @name t\n// ==/UserScript==\nconsole.log(1)".to_string();
        assert!(parse(src).contains("DOMContentLoaded"));
    }

    #[test]
    fn respects_run_at_document_start() {
        let src = "// ==UserScript==\n// @run-at document-start\n// ==/UserScript==\nconsole.log(1)".to_string();
        assert!(!parse(src).contains("DOMContentLoaded"));
    }

    #[test]
    fn missing_directory_yields_empty_not_panic() {
        let missing = std::env::temp_dir().join("glorp-userscripts-does-not-exist");
        assert!(load_all(&missing).is_empty());
    }

    #[test]
    fn reads_only_js_files() {
        let dir = std::env::temp_dir().join("glorp-userscripts-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.js"), "console.log(1)").unwrap();
        fs::write(dir.join("b.JS"), "console.log(2)").unwrap();
        fs::write(dir.join("readme.txt"), "not a script").unwrap();

        assert_eq!(load_all(&dir).len(), 2, "expected both .js and .JS, and no .txt");

        fs::remove_dir_all(&dir).ok();
    }
}
