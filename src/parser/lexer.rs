use std::env;
use std::iter::Peekable;
use std::path::PathBuf;
use std::process::Command;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Pipe,
    And,
    Or,
    Semicolon,
    RedirectOut,
    RedirectAppend,
    RedirectIn,
    RedirectErr,
    RedirectErrAppend,
    RedirectAll,
    Background,
}

pub fn tokenize(input: &str, last_status: i32) -> Result<Vec<Token>, String> {
    let clean_input = input.replace('\r', "");
    let mut tokens = Vec::new();
    let mut chars = clean_input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => {
                chars.next();
            }
            '#' => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    tokens.push(Token::Pipe);
                }
            }
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::And);
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::RedirectAll);
                } else {
                    tokens.push(Token::Background);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::RedirectAppend);
                } else if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::RedirectAll);
                } else {
                    tokens.push(Token::RedirectOut);
                }
            }
            '<' => {
                chars.next();
                tokens.push(Token::RedirectIn);
            }
            '2' if is_peek_redirect_after_two(&chars) => {
                chars.next();
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::RedirectErrAppend);
                } else {
                    tokens.push(Token::RedirectErr);
                }
            }
            _ => tokens.push(Token::Word(read_word(&mut chars, last_status)?)),
        }
    }

    Ok(tokens)
}

fn is_peek_redirect_after_two(chars: &Peekable<Chars<'_>>) -> bool {
    let mut clone = chars.clone();
    clone.next();
    clone.peek() == Some(&'>')
}

fn read_word(chars: &mut Peekable<Chars<'_>>, last_status: i32) -> Result<String, String> {
    let mut word = String::new();
    let mut started = false;
    let mut expand_leading_tilde = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\r' | '\n' | '|' | '&' | ';' | '<' | '>' => break,
            '`' => {
                started = true;
                chars.next();
                let mut subcmd = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '`' {
                        closed = true;
                        break;
                    }
                    subcmd.push(c);
                }
                if !closed {
                    return Err("bnb: syntax error: unmatched backtick".into());
                }
                let output = execute_command_substitution(&subcmd)?;
                for expanded_char in output.chars() {
                    push_quoted_char(&mut word, expanded_char);
                }
            }
            '\'' => {
                started = true;
                chars.next();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '\'' {
                        closed = true;
                        break;
                    }
                    push_quoted_char(&mut word, c);
                }
                if !closed {
                    return Err("bnb: syntax error: unmatched single quote".into());
                }
            }
            '"' => {
                started = true;
                chars.next();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match chars.peek().copied() {
                            Some('"') | Some('\\') | Some('$') | Some('`') => {
                                word.push(chars.next().unwrap());
                            }
                            Some('\n') => {
                                chars.next();
                            }
                            _ => word.push('\\'),
                        },
                        '`' => {
                            let mut subcmd = String::new();
                            let mut inner_closed = false;
                            while let Some(sub_c) = chars.next() {
                                if sub_c == '`' {
                                    inner_closed = true;
                                    break;
                                }
                                subcmd.push(sub_c);
                            }
                            if !inner_closed {
                                return Err("bnb: syntax error: unmatched backtick".into());
                            }
                            let output = execute_command_substitution(&subcmd)?;
                            for expanded_char in output.chars() {
                                push_quoted_char(&mut word, expanded_char);
                            }
                        }
                        '$' => {
                            let mut expanded = String::new();
                            expand_variable(chars, last_status, &mut expanded)?;
                            for expanded_char in expanded.chars() {
                                push_quoted_char(&mut word, expanded_char);
                            }
                        }
                        _ => push_quoted_char(&mut word, c),
                    }
                }
                if !closed {
                    return Err("bnb: syntax error: unmatched double quote".into());
                }
            }
            '\\' => {
                started = true;
                chars.next();
                match chars.next() {
                    Some('\n') => {}
                    Some(c) => push_quoted_char(&mut word, c),
                    None => return Err("bnb: syntax error: trailing escape".into()),
                }
            }
            '$' => {
                started = true;
                if word.is_empty() {
                    expand_leading_tilde = false;
                }
                chars.next();
                expand_variable(chars, last_status, &mut word)?;
            }
            _ => {
                if !started && ch == '~' {
                    expand_leading_tilde = true;
                }
                started = true;
                word.push(ch);
                chars.next();
            }
        }
    }

    if expand_leading_tilde {
        Ok(expand_tilde(&word))
    } else {
        Ok(word)
    }
}

fn push_quoted_char(word: &mut String, ch: char) {
    match ch {
        '*' => word.push('\u{e000}'),
        '?' => word.push('\u{e001}'),
        '{' => word.push('\u{e002}'),
        '}' => word.push('\u{e003}'),
        _ => {}
    }
    if matches!(ch, '*' | '?' | '{' | '}') {
        return;
    }
    word.push(ch);
}

fn execute_command_substitution(cmd: &str) -> Result<String, String> {
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("bnb"));
    let output = Command::new(exe)
        .args(["-c", cmd])
        .output()
        .map_err(|e| format!("bnb: command substitution failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim_end_matches('\n').trim_end_matches('\r');
    Ok(trimmed.to_string())
}

fn expand_variable(
    chars: &mut Peekable<Chars<'_>>,
    last_status: i32,
    result: &mut String,
) -> Result<(), String> {
    match chars.peek().copied() {
        Some('?') => {
            chars.next();
            result.push_str(&last_status.to_string());
        }
        Some('$') => {
            chars.next();
            result.push_str(&std::process::id().to_string());
        }
        Some('(') => {
            chars.next();
            let mut subcmd = String::new();
            let mut depth = 1;
            let mut in_single = false;
            let mut in_double = false;
            let mut closed = false;

            while let Some(c) = chars.next() {
                if c == '\'' && !in_double {
                    in_single = !in_single;
                } else if c == '"' && !in_single {
                    in_double = !in_double;
                } else if !in_single && !in_double {
                    if c == '(' {
                        depth += 1;
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            closed = true;
                            break;
                        }
                    }
                }
                subcmd.push(c);
            }

            if !closed {
                return Err("bnb: syntax error: unmatched '(' in command substitution".into());
            }

            let output = execute_command_substitution(&subcmd)?;
            result.push_str(&output);
        }
        Some('{') => {
            chars.next();
            let mut var_name = String::new();
            let mut closed = false;
            for c in chars.by_ref() {
                if c == '}' {
                    closed = true;
                    break;
                }
                var_name.push(c);
            }
            if !closed {
                return Err("bnb: syntax error: unmatched parameter expansion".into());
            }
            result.push_str(&expand_braced_parameter(&var_name, last_status)?);
        }
        Some(c) if c.is_alphanumeric() || c == '_' => {
            let mut var_name = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' {
                    var_name.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }
            result.push_str(&env::var(&var_name).unwrap_or_default());
        }
        _ => result.push('$'),
    }
    Ok(())
}

fn expand_braced_parameter(expression: &str, last_status: i32) -> Result<String, String> {
    if expression == "?" {
        return Ok(last_status.to_string());
    }
    if expression == "$" {
        return Ok(std::process::id().to_string());
    }

    if let Some(name) = expression.strip_prefix('#') {
        validate_parameter_name(name)?;
        let value = env::var(name).unwrap_or_default();
        return Ok(value.chars().count().to_string());
    }

    let (name, operator, operand) = split_parameter_operator(expression);
    validate_parameter_name(name)?;
    let value = env::var(name).ok();
    let is_set = value.is_some();
    let value = value.unwrap_or_default();
    let is_null = value.is_empty();

    match operator {
        Some(":-") => Ok(if !is_set || is_null {
            operand.to_string()
        } else {
            value
        }),
        Some("-") => Ok(if !is_set { operand.to_string() } else { value }),
        Some(":+") => Ok(if is_set && !is_null {
            operand.to_string()
        } else {
            String::new()
        }),
        Some("+") => Ok(if is_set {
            operand.to_string()
        } else {
            String::new()
        }),
        Some(":=") => {
            if !is_set || is_null {
                env::set_var(name, operand);
                Ok(operand.to_string())
            } else {
                Ok(value)
            }
        }
        Some("=") => {
            if !is_set {
                env::set_var(name, operand);
                Ok(operand.to_string())
            } else {
                Ok(value)
            }
        }
        Some(":?") => {
            if !is_set || is_null {
                Err(parameter_error(name, operand))
            } else {
                Ok(value)
            }
        }
        Some("?") => {
            if !is_set {
                Err(parameter_error(name, operand))
            } else {
                Ok(value)
            }
        }
        Some("#") => Ok(value.strip_prefix(operand).unwrap_or(&value).to_string()),
        Some("##") => Ok(strip_longest_prefix(&value, operand)),
        Some("%") => Ok(value.strip_suffix(operand).unwrap_or(&value).to_string()),
        Some("%%") => Ok(strip_longest_suffix(&value, operand)),
        Some(operator) => Err(format!(
            "bnb: unsupported parameter expansion operator '{}'",
            operator
        )),
        None => Ok(value),
    }
}

fn split_parameter_operator(expression: &str) -> (&str, Option<&str>, &str) {
    let name_end = expression
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(expression.len());
    let (name, remainder) = expression.split_at(name_end);
    for operator in [
        ":-", ":=", ":+", ":?", "##", "%%", "-", "=", "+", "?", "#", "%",
    ] {
        if let Some(operand) = remainder.strip_prefix(operator) {
            return (name, Some(operator), operand);
        }
    }
    (name, None, "")
}

fn validate_parameter_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    if matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err(format!("bnb: bad substitution: {}", name))
    }
}

fn parameter_error(name: &str, message: &str) -> String {
    if message.is_empty() {
        format!("bnb: {}: parameter null or not set", name)
    } else {
        format!("bnb: {}", message)
    }
}

fn strip_longest_prefix(value: &str, pattern: &str) -> String {
    (0..=value.len())
        .rev()
        .filter(|&end| value.is_char_boundary(end))
        .find(|&end| &value[..end] == pattern)
        .map_or_else(|| value.to_string(), |end| value[end..].to_string())
}

fn strip_longest_suffix(value: &str, pattern: &str) -> String {
    (0..=value.len())
        .filter(|&start| value.is_char_boundary(start))
        .find(|&start| &value[start..] == pattern)
        .map_or_else(|| value.to_string(), |start| value[..start].to_string())
}

fn expand_tilde(word: &str) -> String {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| "~".into());

    if word == "~" {
        home
    } else if word.starts_with("~/") || word.starts_with("~\\") {
        format!("{}{}", home, &word[1..])
    } else {
        word.to_string()
    }
}