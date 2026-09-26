use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct ZEntry {
    path: String,
    weight: f64,
    time: u64,
}

fn get_z_file() -> Option<PathBuf> {
    env::var("HOME").ok().map(|h| PathBuf::from(h).join(".bnb_zdata"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_entries() -> Vec<ZEntry> {
    let mut entries = Vec::new();
    let file_path = match get_z_file() {
        Some(p) => p,
        None => return entries,
    };

    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 3 {
                if let (Ok(w), Ok(t)) = (parts[1].parse::<f64>(), parts[2].parse::<u64>()) {
                    entries.push(ZEntry {
                        path: parts[0].to_string(),
                        weight: w,
                        time: t,
                    });
                }
            }
        }
    }
    entries
}

fn save_entries(entries: &[ZEntry]) {
    let file_path = match get_z_file() {
        Some(p) => p,
        None => return,
    };

    if let Ok(mut file) = File::create(file_path) {
        for e in entries {
            let _ = writeln!(file, "{}|{}|{}", e.path, e.weight, e.time);
        }
    }
}

pub fn add_path(path: &Path) {
    let path_str = match path.to_str() {
        Some(s) => s,
        None => return,
    };

    let mut entries = load_entries();
    let now = now_secs();
    let mut found = false;
    let mut total_weight = 0.0;

    for e in &mut entries {
        if e.path == path_str {
            e.weight += 1.0;
            e.time = now;
            found = true;
        }
        total_weight += e.weight;
    }

    if !found {
        entries.push(ZEntry {
            path: path_str.to_string(),
            weight: 1.0,
            time: now,
        });
        total_weight += 1.0;
    }

    if total_weight > 9000.0 {
        for e in &mut entries {
            e.weight *= 0.9;
        }
    }

    save_entries(&entries);
}

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        let entries = load_entries();
        for e in entries {
            println!("{:<10.1} {}", e.weight, e.path);
        }
        return Ok(());
    }

    let query = args.join(" ").to_lowercase();
    let entries = load_entries();
    let now = now_secs();

    let mut best_score = -1.0;
    let mut best_path: Option<String> = None;

    for e in entries {
        let path_lower = e.path.to_lowercase();
        if path_lower.contains(&query) {
            let age_hours = ((now.saturating_sub(e.time)) as f64) / 3600.0;
            let rank = if age_hours < 1.0 {
                e.weight * 4.0
            } else if age_hours < 24.0 {
                e.weight * 2.0
            } else if age_hours < 168.0 {
                e.weight / 2.0
            } else {
                e.weight / 4.0
            };

            if rank > best_score {
                best_score = rank;
                best_path = Some(e.path.clone());
            }
        }
    }

    if let Some(target) = best_path {
        let path = PathBuf::from(&target);
        env::set_current_dir(&path)
            .map_err(|e| format!("z: cd failed to {}: {}", target, e))?;
        println!("{}", target);
        Ok(())
    } else {
        Err(format!("z: no match found for '{}'", query))
    }
}
