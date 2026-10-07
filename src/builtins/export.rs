use std::env;
use std::io::Write;

pub fn expand_env(val: &str) -> String {
    let mut result = String::new();
    let mut chars = val.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('{') => {
                chars.next();
                let mut name = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '}' {
                        closed = true;
                        break;
                    }
                    name.push(c);
                }
                if closed {
                    result.push_str(&env::var(name).unwrap_or_default());
                } else {
                    result.push_str("${");
                    result.push_str(&name);
                }
            }
            Some(first) if first == '_' || first.is_ascii_alphabetic() => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '_' || c.is_ascii_alphanumeric() {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                result.push_str(&env::var(name).unwrap_or_default());
            }
            _ => result.push('$'),
        }
    }

    result
}

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    if args.is_empty() {
        let mut vars: Vec<(String, String)> = env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in vars {
            let _ = writeln!(out, "export {}=\"{}\"", k, v);
        }
        return Ok(());
    }

    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            let clean_key = key.trim();
            if !valid_identifier(clean_key) {
                return Err(format!("export: not a valid identifier: {}", clean_key));
            }
            if value.contains('\0') {
                return Err(format!("export: invalid value for {}", clean_key));
            }
            let clean_val = value.trim().trim_matches(|c| c == '\'' || c == '"');
            let expanded = expand_env(clean_val);
            env::set_var(clean_key, expanded);
        } else {
            if env::var(arg).is_err() {
                env::set_var(arg, "");
            }
        }
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
    fn expands_parameters_without_replacing_substrings() {
        env::set_var("BNB_EXPAND_TEST", "value");
        assert_eq!(
            expand_env("$BNB_EXPAND_TEST/${BNB_EXPAND_TEST}_suffix"),
            "value/value_suffix"
        );
        env::remove_var("BNB_EXPAND_TEST");
    }
}
