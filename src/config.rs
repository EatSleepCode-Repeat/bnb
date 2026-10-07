use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn get_home_dir() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

pub fn load_config() -> Result<(), String> {
    if let Some(home) = get_home_dir() {
        parse_if_present(&home.join(".bnbrc"))
    } else {
        Ok(())
    }
}

pub fn parse_file(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("source: cannot read {}: {}", path.display(), e))?;
    let mut unsupported = Vec::new();

    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let result = if let Some(kv) = trimmed.strip_prefix("export ") {
            parse_export(kv)
        } else if let Some(kv) = trimmed.strip_prefix("alias ") {
            parse_alias(kv)
        } else {
            Err("unsupported statement (supported: export KEY=VALUE, alias NAME=VALUE)".into())
        };

        if let Err(reason) = result {
            unsupported.push(format!(
                "{}:{}: {}",
                path.display(),
                line_number + 1,
                reason
            ));
        }
    }

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(unsupported.join("\n"))
    }
}

fn parse_if_present(path: &Path) -> Result<(), String> {
    match fs::metadata(path) {
        Ok(_) => parse_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "config: cannot inspect {}: {}",
            path.display(),
            error
        )),
    }
}

fn parse_export(kv: &str) -> Result<(), String> {
    let (key, value) = kv
        .split_once('=')
        .ok_or_else(|| "expected export KEY=VALUE".to_string())?;
    let key = key.trim();
    if !valid_identifier(key) {
        return Err(format!("invalid environment variable name: {}", key));
    }

    let (value, expand) = unquote(value.trim())?;
    validate_value(&value)?;
    let value = if expand {
        crate::builtins::export::expand_env(&value)
    } else {
        value
    };
    env::set_var(key, value);
    Ok(())
}

fn parse_alias(kv: &str) -> Result<(), String> {
    let (key, value) = kv
        .split_once('=')
        .ok_or_else(|| "expected alias NAME=VALUE".to_string())?;
    let key = key.trim();
    if !valid_identifier(key) {
        return Err(format!("invalid alias name: {}", key));
    }

    let (value, _) = unquote(value.trim())?;
    validate_value(&value)?;
    crate::builtins::alias::add_alias(key, &value);
    Ok(())
}

fn unquote(value: &str) -> Result<(String, bool), String> {
    if value.starts_with('"') || value.starts_with('\'') {
        let quote = value.chars().next().unwrap();
        if value.len() < 2 || !value.ends_with(quote) {
            return Err("unmatched quote".into());
        }
        let inner = &value[1..value.len() - 1];
        let unescaped = if quote == '"' {
            unescape_double_quoted(inner)
        } else {
            inner.to_string()
        };
        Ok((unescaped, quote == '"'))
    } else {
        if value.contains('"') || value.contains('\'') {
            return Err("quotes must surround the complete value".into());
        }
        Ok((value.to_string(), true))
    }
}

fn unescape_double_quoted(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('"') | Some('\\') | Some('$') => {
                out.push(chars.next().unwrap());
            }
            Some('n') => {
                out.push('\n');
                chars.next();
            }
            Some('r') => {
                out.push('\r');
                chars.next();
            }
            Some('t') => {
                out.push('\t');
                chars.next();
            }
            _ => out.push('\\'),
        }
    }

    out
}

fn validate_value(value: &str) -> Result<(), String> {
    if value.contains('\0') {
        return Err("NUL bytes are not valid in config values".into());
    }
    if value.contains("$(") || value.contains('`') {
        return Err("command substitution is not supported in config values".into());
    }
    if value.contains("${") && !value.contains('}') {
        return Err("unmatched parameter expansion".into());
    }
    Ok(())
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_values_require_balanced_outer_quotes() {
        assert_eq!(
            unquote("\"hello world\"").unwrap(),
            ("hello world".into(), true)
        );
        assert_eq!(unquote("'hello'").unwrap(), ("hello".into(), false));
        assert_eq!(
            unquote("\"a\\n\\t\\$HOME\"").unwrap(),
            ("a\n\t$HOME".into(), true)
        );
        assert!(unquote("\"unfinished").is_err());
        assert!(unquote("hello\"world").is_err());
    }

    #[test]
    fn config_rejects_shell_execution_and_malformed_variables() {
        assert!(validate_value("$(touch /tmp/unexpected)").is_err());
        assert!(validate_value("`whoami`").is_err());
        assert!(validate_value("${HOME").is_err());
        assert!(validate_value("$HOME/bin").is_ok());
    }

    #[test]
    fn config_applies_supported_lines_and_reports_unsupported_ones() {
        let suffix = std::process::id();
        let variable = format!("BNB_CONFIG_TEST_{}", suffix);
        let alias = format!("bnb_config_test_{}", suffix);
        let path = env::temp_dir().join(format!("bnb-config-test-{}.rc", suffix));
        fs::write(
            &path,
            format!(
                "export {}='configured value'\nunsupported command\nalias {}='echo configured'\n",
                variable, alias
            ),
        )
        .unwrap();

        let error = parse_file(&path).unwrap_err();
        assert!(error.contains(":2: unsupported statement"));
        assert_eq!(env::var(&variable).unwrap(), "configured value");
        assert_eq!(
            crate::builtins::alias::resolve(&alias).as_deref(),
            Some("echo configured")
        );

        env::remove_var(variable);
        crate::builtins::alias::remove_alias(&alias);
        fs::remove_file(path).unwrap();
    }
}
