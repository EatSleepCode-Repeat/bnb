use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn get_home_dir() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

pub fn load_config() {
    if let Some(home) = get_home_dir() {
        let bnbrc = home.join(".bnbrc");
        parse_file(&bnbrc);

        if cfg!(not(target_os = "windows")) {
            let zshrc = home.join(".zshrc");
            parse_file(&zshrc);
            let bashrc = home.join(".bashrc");
            parse_file(&bashrc);
        }
    }
}

fn parse_file(path: &PathBuf) {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("export ") {
                let kv = &trimmed[7..];
                if let Some((key, val)) = kv.split_once('=') {
                    let clean_key = key.trim();
                    let clean_val = val.trim().trim_matches(|c| c == '\'' || c == '"');
                    let expanded = crate::builtins::export::expand_env(clean_val);
                    env::set_var(clean_key, expanded);
                }
            } else if trimmed.starts_with("alias ") {
                let kv = &trimmed[6..];
                if let Some((key, val)) = kv.split_once('=') {
                    let clean_key = key.trim();
                    let clean_val = val.trim().trim_matches(|c| c == '\'' || c == '"');
                    let expanded = crate::builtins::export::expand_env(clean_val);
                    crate::builtins::alias::add_alias(clean_key, &expanded);
                }
            }
        }
    }
}
