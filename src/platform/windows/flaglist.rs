use std::{fs, io::Write};

use crate::shared::flaglist;

const EXAMPLE_FLAGS: &str = r#"
{
    "flags": [
        "--disable-gpu-vsync"
    ],
    "disabled_defaults": [
        ""
    ]
}"#;

// normalized for CoreWebView2EnvironmentOptions::set_additional_browser_arguments
fn join(flags: Vec<String>) -> String {
    let mut args_str = String::new();
    for flag in flaglist::filter_for_platform(flags) {
        args_str = args_str + &flag + " ";
    }
    args_str
}

pub fn load() -> String {
    let defaults = flaglist::defaults();
    let flaglist_path = crate::shared::paths::settings_dir().join("user_flags.json");

    let mut flaglist_file = if let Ok(flaglist_file) = fs::OpenOptions::new().write(true).read(true).create(true).truncate(false).open(&flaglist_path)
    {
        flaglist_file
    } else {
        eprintln!("can't open user flags file");
        return join(defaults);
    };

    if flaglist_file.metadata().unwrap().len() == 0 {
        flaglist_file.write_all(EXAMPLE_FLAGS.as_bytes()).ok();
    }

    let flaglist_string = if let Ok(flaglist_string) = fs::read_to_string(&flaglist_path) {
        flaglist_string
    } else {
        eprintln!("can't read user flags file");
        flaglist_file.set_len(0).ok();
        flaglist_file.write_all(EXAMPLE_FLAGS.as_bytes()).ok();
        return join(defaults);
    };

    let user = match flaglist::try_parse(&flaglist_string) {
        Ok(config) => config,
        Err(_) => {
            flaglist_file.set_len(0).ok();
            flaglist_file.write_all(EXAMPLE_FLAGS.as_bytes()).ok();
            return join(defaults);
        }
    };

    join(flaglist::merge(defaults, &user))
}
