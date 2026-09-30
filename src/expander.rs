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
    if let (Some(start), Some(end)) = (arg.find('{'), arg.rfind('}')) {
        if start < end {
            let prefix = &arg[..start];
            let suffix = &arg[end + 1..];
            let inner = &arg[start + 1..end];

            // Range expansion: e.g. {1..5} or {5..1} or {a..e}
            if let Some((start_s, end_s)) = inner.split_once("..") {
                if let (Ok(start_num), Ok(end_num)) =
                    (start_s.trim().parse::<i64>(), end_s.trim().parse::<i64>())
                {
                    let mut items = Vec::new();
                    if start_num <= end_num {
                        for n in start_num..=end_num {
                            items.push(format!("{}{}{}", prefix, n, suffix));
                        }
                    } else {
                        for n in (end_num..=start_num).rev() {
                            items.push(format!("{}{}{}", prefix, n, suffix));
                        }
                    }
                    return items;
                } else if start_s.len() == 1 && end_s.len() == 1 {
                    let c1 = start_s.chars().next().unwrap();
                    let c2 = end_s.chars().next().unwrap();
                    if (c1.is_ascii_lowercase() && c2.is_ascii_lowercase())
                        || (c1.is_ascii_uppercase() && c2.is_ascii_uppercase())
                    {
                        let mut items = Vec::new();
                        if c1 <= c2 {
                            for c in (c1 as u8)..=(c2 as u8) {
                                items.push(format!("{}{}{}", prefix, c as char, suffix));
                            }
                        } else {
                            for c in ((c2 as u8)..=(c1 as u8)).rev() {
                                items.push(format!("{}{}{}", prefix, c as char, suffix));
                            }
                        }
                        return items;
                    }
                }
            }

            // Comma separated: e.g. {foo,bar,baz}
            if inner.contains(',') {
                return inner
                    .split(',')
                    .map(|item| format!("{}{}{}", prefix, item.trim(), suffix))
                    .collect();
            }
        }
    }
    vec![arg.to_string()]
}

pub fn expand_globs(arg: &str) -> Vec<String> {
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
                let full = if dir == *"." {
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
        (Some(b'*'), Some(_)) => {
            glob_match(&pat[1..], text) || glob_match(pat, &text[1..])
        }
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
    fn test_match_pattern() {
        assert!(match_pattern("*.rs", "main.rs"));
        assert!(match_pattern("*test*", "my_test_runner.rs"));
        assert!(match_pattern("foo?.txt", "foo1.txt"));
        assert!(!match_pattern("foo?.txt", "foo12.txt"));
        assert!(!match_pattern("*.rs", ".hidden.rs"));
        assert!(match_pattern(".*.rs", ".hidden.rs"));
    }
}
