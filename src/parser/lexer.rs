use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Pipe,              // |
    And,               // &&
    Or,                // ||
    Semicolon,         // ;
    RedirectOut,       // >
    RedirectAppend,    // >>
    RedirectIn,        // <
    RedirectErr,       // 2>
    RedirectErrAppend, // 2>>
    RedirectAll,       // &>
    Background,        // &
}

pub fn tokenize(input: &str, last_status: i32) -> Vec<Token> {
    let clean_input = input.replace('\r', "");
    let mut tokens = Vec::new();
    let mut chars = clean_input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => {
                chars.next();
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
            // Check for 2> or 2>> redirection
            '2' if is_peek_redirect_after_two(&chars) => {
                chars.next(); // consume '2'
                chars.next(); // consume '>'
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::RedirectErrAppend);
                } else {
                    tokens.push(Token::RedirectErr);
                }
            }
            _ => {
                let word = read_word(&mut chars, last_status);
                tokens.push(Token::Word(word));
            }
        }
    }

    tokens
}

fn is_peek_redirect_after_two(chars: &std::iter::Peekable<std::str::Chars>) -> bool {
    let mut clone = chars.clone();
    clone.next(); // '2'
    clone.peek() == Some(&'>')
}

fn read_word(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    last_status: i32,
) -> String {
    let mut word = String::new();
    let mut first_char = true;
    let mut started_with_tilde = false;

    while let Some(&ch) = chars.peek() {
        if first_char && ch == '~' {
            started_with_tilde = true;
        }
        first_char = false;

        match ch {
            ' ' | '\t' | '\r' | '\n' | '|' | '&' | ';' | '<' => break,
            '>' => break,
            '\'' => {
                chars.next(); // consume opening single quote
                for c in chars.by_ref() {
                    if c == '\'' {
                        break;
                    }
                    word.push(c);
                }
            }
            '"' => {
                chars.next(); // consume opening double quote
                while let Some(c) = chars.next() {
                    if c == '"' {
                        break;
                    }
                    if c == '\\' {
                        if let Some(&next_c) = chars.peek() {
                            if next_c == '"' || next_c == '\\' || next_c == '$' || next_c == '`' {
                                chars.next();
                                word.push(next_c);
                                continue;
                            }
                        }
                        word.push('\\');
                    } else if c == '$' {
                        expand_variable(chars, last_status, &mut word);
                    } else {
                        word.push(c);
                    }
                }
            }
            '\\' => {
                chars.next(); // consume backslash
                if let Some(c) = chars.next() {
                    word.push(c);
                }
            }
            '$' => {
                chars.next(); // consume '$'
                expand_variable(chars, last_status, &mut word);
            }
            _ => {
                word.push(ch);
                chars.next();
            }
        }
    }

    if started_with_tilde {
        expand_tilde(&word)
    } else {
        word
    }
}

fn expand_variable(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    last_status: i32,
    result: &mut String,
) {
    match chars.peek() {
        Some(&'?') => {
            chars.next();
            result.push_str(&last_status.to_string());
        }
        Some(&'$') => {
            chars.next();
            result.push_str(&std::process::id().to_string());
        }
        Some(&'{') => {
            chars.next(); // consume '{'
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == '}' {
                    break;
                }
                var_name.push(c);
            }
            if var_name == "?" {
                result.push_str(&last_status.to_string());
            } else if var_name == "$" {
                result.push_str(&std::process::id().to_string());
            } else if let Ok(val) = env::var(&var_name) {
                result.push_str(&val);
            }
        }
        Some(&c) if c.is_alphanumeric() || c == '_' => {
            let mut var_name = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' {
                    var_name.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(val) = env::var(&var_name) {
                result.push_str(&val);
            }
        }
        _ => {
            result.push('$');
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quotes_and_words() {
        let tokens = tokenize("echo 'hello $USER' \"hi $USER\"", 0);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Word("echo".to_string()));
        assert_eq!(tokens[1], Token::Word("hello $USER".to_string()));
    }

    #[test]
    fn test_exit_status_expansion() {
        let tokens = tokenize("echo $?", 42);
        assert_eq!(tokens[1], Token::Word("42".to_string()));
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("cargo check && cargo test || echo fail ; ls", 0);
        assert_eq!(tokens[2], Token::And);
        assert_eq!(tokens[5], Token::Or);
        assert_eq!(tokens[8], Token::Semicolon);
    }

    #[test]
    fn test_assignment_with_quotes() {
        let tokens = tokenize("export FOO=\"hello world\"", 0);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Word("export".to_string()));
        assert_eq!(tokens[1], Token::Word("FOO=hello world".to_string()));
    }
}