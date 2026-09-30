use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn get_home_dir() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

pub fn load_config() {
    if let Some(home) = get_home_dir() {
        let bnbrc = home.join(".bnbrc");
        let _ = parse_file(&bnbrc);

        if cfg!(not(target_os = "windows")) {
            let zshrc = home.join(".zshrc");
            let _ = parse_file(&zshrc);
            let bashrc = home.join(".bashrc");
            let _ = parse_file(&bashrc);
        }
    }
}

pub fn parse_file(path: &Path) -> Result<(), String> {
    let file = File::open(path)
        .map_err(|e| format!("source: cannot read {}: {}", path.display(), e))?;

    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(kv) = trimmed.strip_prefix("export ") {
            if let Some((key, val)) = kv.split_once('=') {
                let clean_key = key.trim();
                let clean_val = val.trim().trim_matches(|c| c == '\'' || c == '"');
                let expanded = crate::builtins::export::expand_env(clean_val);
                env::set_var(clean_key, expanded);
            }
        } else if let Some(kv) = trimmed.strip_prefix("alias ") {
            if let Some((key, val)) = kv.split_once('=') {
                let clean_key = key.trim();
                let clean_val = val.trim().trim_matches(|c| c == '\'' || c == '"');
                let expanded = crate::builtins::export::expand_env(clean_val);
                crate::builtins::alias::add_alias(clean_key, &expanded);
            }
        }
    }
    Ok(())
}
