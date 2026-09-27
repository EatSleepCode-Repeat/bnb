use std::env;
use std::path::PathBuf;

pub fn expand_args(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();

    for arg in args {
        let tilde_expanded = expand_tilde(arg);
        let brace_expanded = expand_braces(&tilde_expanded);

        for item in brace_expanded {
            let glob_expanded = expand_globs(&item);
            result.extend(glob_expanded);
        }
    }

    result
}

fn expand_tilde(arg: &str) -> String {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();

    if home.is_empty() {
        return arg.to_string();
    }

    if arg == "~" {
        home
    } else if arg.starts_with("~/") || arg.starts_with("~\\") {
        format!("{}{}", home, &arg[1..])
    } else {
        arg.to_string()
    }
}

fn expand_braces(arg: &str) -> Vec<String> {
    if let (Some(start), Some(end)) = (arg.find('{'), arg.rfind('}')) {
        if start < end {
            let prefix = &arg[..start];
            let suffix = &arg[end + 1..];
            let inner = &arg[start + 1..end];

            return inner
                .split(',')
                .map(|item| format!("{}{}{}", prefix, item.trim(), suffix))
                .collect();
        }
    }
    vec![arg.to_string()]
}

fn expand_globs(arg: &str) -> Vec<String> {
    if !arg.contains('*') && !arg.contains('?') {
        return vec![arg.to_string()];
    }

    let path = PathBuf::from(arg);
    let (dir, pattern) = if let Some(parent) = path.parent() {
        let d = if parent.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            parent.to_path_buf()
        };
        let p = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        (d, p)
    } else {
        (PathBuf::from("."), arg.to_string())
    };

    let mut matches = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if match_pattern(&pattern, &name) {
                let full = if dir == PathBuf::from(".") {
                    name
                } else {
                    dir.join(name).to_string_lossy().to_string()
                };
                matches.push(full);
            }
        }
    }

    if matches.is_empty() {
        vec![arg.to_string()]
    } else {
        matches.sort();
        matches
    }
}

fn match_pattern(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return !name.starts_with('.');
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        return name.ends_with(suffix);
    }

    pattern == name
}
