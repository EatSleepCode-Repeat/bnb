use std::collections::HashMap;
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

pub fn resolve(cmd: &str) -> Option<String> {
    let guard = ALIASES.lock().unwrap();
    if let Some(ref map) = *guard {
        map.get(cmd).cloned()
    } else {
        None
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        let guard = ALIASES.lock().unwrap();
        if let Some(ref map) = *guard {
            for (k, v) in map {
                println!("alias {}='{}'", k, v);
            }
        }
        return Ok(());
    }

    for arg in args {
        if let Some((key, val)) = arg.split_once('=') {
            let clean_val = val.trim_matches(|c| c == '\'' || c == '"');
            add_alias(key, clean_val);
        }
    }
    Ok(())
}
