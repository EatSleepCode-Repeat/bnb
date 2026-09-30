use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct ZEntry {
    path: String,
    weight: f64,
    time: u64,
}

fn get_z_file() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".bnb_zdata"))
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
        for line in reader.lines().map_while(Result::ok) {
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

fn calculate_rank(weight: f64, time: u64, now: u64) -> f64 {
    let age_hours = ((now.saturating_sub(time)) as f64) / 3600.0;
    if age_hours < 1.0 {
        weight * 4.0
    } else if age_hours < 24.0 {
        weight * 2.0
    } else if age_hours < 168.0 {
        weight / 2.0
    } else {
        weight / 4.0
    }
}

fn change_to_path(target: &str) -> Result<(), String> {
    let path = PathBuf::from(target);
    let old_pwd = env::current_dir().ok();

    env::set_current_dir(&path).map_err(|e| format!("z: cd failed to {}: {}", target, e))?;

    if let Some(old) = old_pwd {
        if let Some(old_str) = old.to_str() {
            env::set_var("OLDPWD", old_str);
        }
    }
    if let Ok(new_pwd) = env::current_dir() {
        if let Some(pwd_str) = new_pwd.to_str() {
            env::set_var("PWD", pwd_str);
        }
        add_path(&new_pwd);
    }
    println!("{}", target);
    Ok(())
}

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        let entries = load_entries();
        for e in entries {
            println!("{:<10.1} {}", e.weight, e.path);
        }
        return Ok(());
    }

    let is_interactive = args[0] == "-i";
    let query_terms: Vec<String> = if is_interactive {
        args[1..].iter().map(|s| s.to_lowercase()).collect()
    } else {
        args.iter().map(|s| s.to_lowercase()).collect()
    };

    let entries = load_entries();
    let now = now_secs();

    let mut ranked: Vec<(f64, String)> = entries
        .into_iter()
        .filter_map(|e| {
            let path_lower = e.path.to_lowercase();
            let matches = query_terms.is_empty()
                || query_terms.iter().all(|term| path_lower.contains(term));
            if matches {
                let rank = calculate_rank(e.weight, e.time, now);
                Some((rank, e.path))
            } else {
                None
            }
        })
        .collect();

    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if ranked.is_empty() {
        let q_str = query_terms.join(" ");
        return Err(format!("z: no match found for '{}'", q_str));
    }

    if is_interactive {
        let count = ranked.len().min(10);
        println!("\x1b[1mRanked directory matches:\x1b[0m");
        for (i, (rank, path)) in ranked[..count].iter().enumerate() {
            println!("  \x1b[1;36m{:2})\x1b[0m \x1b[33m[{:5.1}]\x1b[0m {}", i + 1, rank, path);
        }
        print!("\x1b[1mSelect [1-{}, q to cancel]: \x1b[0m", count);
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim();
            if trimmed.eq_ignore_ascii_case("q") || trimmed.is_empty() {
                return Ok(());
            }
            if let Ok(choice) = trimmed.parse::<usize>() {
                if choice >= 1 && choice <= count {
                    return change_to_path(&ranked[choice - 1].1);
                }
            }
        }
        return Err("z: invalid selection".to_string());
    }

    change_to_path(&ranked[0].1)
}
