use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DAY_IN_SECS: u64 = 86400;

fn get_cache_file() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".bnb_update_cache"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn check_for_updates_async() {
    thread::spawn(|| {
        let cache_file = match get_cache_file() {
            Some(p) => p,
            None => return,
        };

        let now = now_secs();

        if let Ok(file) = File::open(&cache_file) {
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().flatten().collect();
            if lines.len() >= 2 {
                if let Ok(last_check) = lines[0].parse::<u64>() {
                    if now.saturating_sub(last_check) < DAY_IN_SECS {
                        return;
                    }
                }
            }
        }

        let output = Command::new("curl")
            .args([
                "-s",
                "-m",
                "2",
                "-A",
                "bnb-shell",
                "https://crates.io/api/v1/crates/bnb-shell",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let response = String::from_utf8_lossy(&out.stdout);
                if let Some(pos) = response.find("\"max_stable_version\":\"") {
                    let start = pos + 22;
                    if let Some(end) = response[start..].find('"') {
                        let latest_ver = &response[start..start + end];
                        let _ = fs::write(
                            &cache_file,
                            format!("{}\n{}\n", now, latest_ver),
                        );
                    }
                }
            }
        }
    });
}

pub fn print_update_banner_if_available() {
    let cache_file = match get_cache_file() {
        Some(p) => p,
        None => return,
    };

    if let Ok(file) = File::open(cache_file) {
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().flatten().collect();
        if lines.len() >= 2 {
            let latest_version = lines[1].trim();
            if !latest_version.is_empty() && is_newer(latest_version, CURRENT_VERSION) {
                println!(
                    "\x1b[1;33m⚡ A new version of bnb-shell is available: v{} -> v{}\x1b[0m",
                    CURRENT_VERSION, latest_version
                );
                println!(
                    "\x1b[36m   Run \x1b[1m'cargo install bnb-shell'\x1b[0;36m or \x1b[1m'bnb-update'\x1b[0;36m to upgrade!\x1b[0m\n"
                );
            }
        }
    }
}

pub fn print_version() {
    println!("\x1b[1;36mbnb-shell\x1b[0m v{}", CURRENT_VERSION);
}

pub fn run_update() -> Result<(), String> {
    println!("\x1b[1;32m🚀 Upgrading bnb-shell to the latest version...\x1b[0m");
    let status = Command::new("cargo")
        .args(["install", "bnb-shell", "--force"])
        .status()
        .map_err(|e| format!("bnb-update failed: {}", e))?;

    if status.success() {
        println!("\x1b[1;32m✨ bnb-shell successfully updated! Restart your terminal to apply.\x1b[0m");
        Ok(())
    } else {
        Err("bnb-update: cargo install failed".to_string())
    }
}

fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .collect()
    };
    parse(latest) > parse(current)
}
