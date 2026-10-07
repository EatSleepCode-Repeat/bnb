use std::env;
use std::path::PathBuf;

const MAX_BRACE_EXPANSIONS: usize = 1024;

pub fn expand_args(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();

    for arg in args {
        let tilde_expanded = expand_tilde(arg);
        let brace_expanded = expand_braces(&tilde_expanded);

        for item in brace_expanded {
            let glob_expanded = expand_globs(&item);
            result.extend(
                glob_expanded
                    .iter()
                    .map(|value| restore_quoted_chars(value)),
            );
        }
    }

    result
}

pub fn restore_quoted_chars(arg: &str) -> String {
    arg.chars()
        .map(|ch| match ch {
            '\u{e000}' => '*',
            '\u{e001}' => '?',
            '\u{e002}' => '{',
            '\u{e003}' => '}',
            _ => ch,
        })
        .collect()
}

pub fn expand_tilde(arg: &str) -> String {
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

pub fn expand_braces(arg: &str) -> Vec<String> {
    expand_braces_recursive(arg, 0).unwrap_or_else(|| vec![arg.to_string()])
}

fn expand_braces_recursive(arg: &str, depth: usize) -> Option<Vec<String>> {
    if depth >= 32 {
        return None;
    }

    let Some((start, end)) = find_brace_group(arg) else {
        return Some(vec![arg.to_string()]);
    };
    let prefix = &arg[..start];
    let suffix = &arg[end + 1..];
    let inner = &arg[start + 1..end];
    let alternatives = match expand_brace_range(inner) {
        Some(items) => items,
        None => split_brace_alternatives(inner).unwrap_or_default(),
    };

    if alternatives.is_empty() {
        return Some(vec![arg.to_string()]);
    }

    let mut results = Vec::new();
    for alternative in alternatives {
        let candidate = format!("{}{}{}", prefix, alternative, suffix);
        for expanded in expand_braces_recursive(&candidate, depth + 1)? {
            if results.len() == MAX_BRACE_EXPANSIONS {
                return None;
            }
            results.push(expanded);
        }
    }
    Some(results)
}

fn find_brace_group(input: &str) -> Option<(usize, usize)> {
    let mut opening = None;
    let mut depth = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '{' if ch != '\u{e002}' => {
                if depth == 0 {
                    opening = Some(index);
                }
                depth += 1;
            }
            '}' if ch != '\u{e003}' && depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    return opening.map(|start| (start, index));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_brace_alternatives(input: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(input[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        parts.push(input[start..].trim().to_string());
        Some(parts)
    }
}

fn expand_brace_range(input: &str) -> Option<Vec<String>> {
    let parts: Vec<&str> = input.split("..").collect();
    if !(parts.len() == 2 || parts.len() == 3) {
        return None;
    }

    let step = if parts.len() == 3 {
        parts[2].parse::<u64>().ok()?
    } else {
        1
    };
    if step == 0 {
        return None;
    }

    if let (Ok(start), Ok(end)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
        let distance = (i128::from(end) - i128::from(start)).unsigned_abs();
        let count = distance / u128::from(step) + 1;
        if count > MAX_BRACE_EXPANSIONS as u128 {
            return None;
        }
        let width = parts[0]
            .trim_start_matches('-')
            .len()
            .max(parts[1].trim_start_matches('-').len());
        let direction = if end >= start { 1i128 } else { -1i128 };
        return Some(
            (0..count)
                .map(|index| {
                    let value = i128::from(start) + direction * i128::from(step) * index as i128;
                    if width > 1 {
                        if value < 0 {
                            format!("-{:0width$}", -value, width = width)
                        } else {
                            format!("{:0width$}", value, width = width)
                        }
                    } else {
                        value.to_string()
                    }
                })
                .collect(),
        );
    }

    if parts[0].len() == 1 && parts[1].len() == 1 {
        let start = parts[0].as_bytes()[0];
        let end = parts[1].as_bytes()[0];
        let alphabetic = (start.is_ascii_lowercase() && end.is_ascii_lowercase())
            || (start.is_ascii_uppercase() && end.is_ascii_uppercase());
        if alphabetic {
            let distance = start.abs_diff(end) as u64;
            let count = distance / step + 1;
            if count as usize > MAX_BRACE_EXPANSIONS {
                return None;
            }
            let direction: i128 = if end >= start { 1 } else { -1 };
            return Some(
                (0..count)
                    .map(|index| {
                        (i128::from(start) + direction * i128::from(step) * i128::from(index)) as u8
                            as char
                    })
                    .map(|ch| ch.to_string())
                    .collect(),
            );
        }
    }
    None
}

pub fn expand_globs(arg: &str) -> Vec<String> {
    if !arg.contains('*') && !arg.contains('?') {
        return vec![arg.to_string()];
    }

    let path = PathBuf::from(arg);
    let absolute = path.is_absolute();
    let components: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::RootDir => None,
            std::path::Component::CurDir => Some(".".to_string()),
            std::path::Component::ParentDir => Some("..".to_string()),
            std::path::Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            std::path::Component::Prefix(prefix) => {
                Some(prefix.as_os_str().to_string_lossy().to_string())
            }
        })
        .collect();
    let mut matches = Vec::new();
    let base = if absolute {
        PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
    } else {
        PathBuf::from(".")
    };
    expand_glob_components(&base, &components, 0, &mut matches);
    if !arg.starts_with("./") {
        for matched in &mut matches {
            if let Some(relative) = matched.strip_prefix("./") {
                *matched = relative.to_string();
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

fn expand_glob_components(
    current: &PathBuf,
    components: &[String],
    index: usize,
    matches: &mut Vec<String>,
) {
    if index == components.len() {
        if current.exists() {
            matches.push(current.to_string_lossy().to_string());
        }
        return;
    }

    let component = &components[index];
    if component == "**" {
        if index + 1 < components.len() {
            expand_glob_components(current, components, index + 1, matches);
        }
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
                let path = entry.path();
                if index + 1 == components.len() {
                    matches.push(path.to_string_lossy().to_string());
                }
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    expand_glob_components(&path, components, index, matches);
                }
            }
        }
    } else if !component.contains('*') && !component.contains('?') {
        expand_glob_components(&current.join(component), components, index + 1, matches);
    } else if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if match_pattern(component, &name) {
                expand_glob_components(&entry.path(), components, index + 1, matches);
            }
        }
    }
}

pub fn match_pattern(pattern: &str, name: &str) -> bool {
    // Hidden files must be matched explicitly with a leading '.'
    if name.starts_with('.') && !pattern.starts_with('.') {
        return false;
    }
    glob_match(pattern.as_bytes(), name.as_bytes())
}

fn glob_match(pat: &[u8], text: &[u8]) -> bool {
    match (pat.first(), text.first()) {
        (None, None) => true,
        (Some(b'*'), None) => pat[1..].iter().all(|&c| c == b'*'),
        (Some(b'*'), Some(_)) => glob_match(&pat[1..], text) || glob_match(pat, &text[1..]),
        (Some(b'?'), Some(_)) => glob_match(&pat[1..], &text[1..]),
        (Some(p), Some(t)) if p == t => glob_match(&pat[1..], &text[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_braces_range() {
        let expanded = expand_braces("file{1..3}.txt");
        assert_eq!(expanded, vec!["file1.txt", "file2.txt", "file3.txt"]);

        let expanded_alpha = expand_braces("{a..c}.rs");
        assert_eq!(expanded_alpha, vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn test_expand_braces_comma() {
        let expanded = expand_braces("pre_{a,b,c}_post");
        assert_eq!(expanded, vec!["pre_a_post", "pre_b_post", "pre_c_post"]);
    }

    #[test]
    fn test_expand_nested_and_stepped_braces() {
        assert_eq!(
            expand_braces("item{a,{c,e}}"),
            vec!["itema", "itemc", "iteme"]
        );
        assert_eq!(expand_braces("n{01..07..3}"), vec!["n01", "n04", "n07"]);
        assert_eq!(expand_braces("{e..a..2}"), vec!["e", "c", "a"]);
    }

    #[test]
    fn test_recursive_globs_match_nested_files_without_hidden_directories() {
        let root = env::temp_dir().join(format!("bnb-glob-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::write(root.join("a/one.rs"), "").unwrap();
        std::fs::write(root.join("a/b/two.rs"), "").unwrap();
        std::fs::write(root.join(".hidden/secret.rs"), "").unwrap();

        let pattern = format!("{}/**/*.rs", root.display());
        let mut matches = expand_globs(&pattern);
        matches.sort();
        assert_eq!(
            matches,
            vec![
                root.join("a/b/two.rs").to_string_lossy().to_string(),
                root.join("a/one.rs").to_string_lossy().to_string()
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_match_pattern() {
        assert!(match_pattern("*.rs", "main.rs"));
        assert!(match_pattern("*test*", "my_test_runner.rs"));
        assert!(match_pattern("foo?.txt", "foo1.txt"));
        assert!(!match_pattern("foo?.txt", "foo12.txt"));
        assert!(!match_pattern("*.rs", ".hidden.rs"));
        assert!(match_pattern(".*.rs", ".hidden.rs"));
    }
}
