use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;

static ALIASES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

pub fn add_alias(key: &str, value: &str) {
    let mut guard = ALIASES.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    if let Some(ref mut map) = *guard {
        map.insert(key.to_string(), value.to_string());
    }
}

pub fn remove_alias(key: &str) -> bool {
    let mut guard = ALIASES.lock().unwrap();
    if let Some(ref mut map) = *guard {
        map.remove(key).is_some()
    } else {
        false
    }
}

pub fn remove_all_aliases() {
    let mut guard = ALIASES.lock().unwrap();
    if let Some(ref mut map) = *guard {
        map.clear();
    }
}

pub fn resolve(cmd: &str) -> Option<String> {
    let guard = ALIASES.lock().unwrap();
    if let Some(ref map) = *guard {
        map.get(cmd).cloned()
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    if args.is_empty() {
        let guard = ALIASES.lock().unwrap();
        if let Some(ref map) = *guard {
            let mut items: Vec<(&String, &String)> = map.iter().collect();
            items.sort_by(|a, b| a.0.cmp(b.0));
            for (k, v) in items {
                let _ = writeln!(out, "alias {}='{}'", k, v);
            }
        }
        return Ok(());
    }

    for arg in args {
        if let Some((key, val)) = arg.split_once('=') {
            let clean_key = key.trim();
            let clean_val = val.trim().trim_matches(|c| c == '\'' || c == '"');
            add_alias(clean_key, clean_val);
        } else if let Some(val) = resolve(arg) {
            let _ = writeln!(out, "alias {}='{}'", arg, val);
        } else {
            return Err(format!("alias: {}: not found", arg));
        }
    }
    Ok(())
}
