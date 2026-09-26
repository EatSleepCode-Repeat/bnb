use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Pipe,
    RedirectOut,
    RedirectAppend,
    RedirectIn,
    Background,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let clean_input = input.replace('\r', "");
    let mut tokens = Vec::new();
    let mut chars = clean_input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => {
                chars.next();
            }
            '|' => {
                tokens.push(Token::Pipe);
                chars.next();
            }
            '&' => {
                tokens.push(Token::Background);
                chars.next();
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::RedirectAppend);
                } else {
                    tokens.push(Token::RedirectOut);
                }
            }
            '<' => {
                tokens.push(Token::RedirectIn);
                chars.next();
            }
            '"' | '\'' => {
                let quote = chars.next().unwrap();
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == quote {
                        break;
                    }
                    word.push(c);
                }
                tokens.push(Token::Word(expand_word(&word)));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if " \t\r\n|&><\"'".contains(c) {
                        break;
                    }
                    word.push(c);
                    chars.next();
                }
                tokens.push(Token::Word(expand_word(&word)));
            }
        }
    }

    tokens
}

pub fn expand_word(word: &str) -> String {
    let expanded_env = expand_env_vars(word);
    expand_tilde(&expanded_env)
}

fn expand_tilde(word: &str) -> String {
    if word == "~" {
        env::var("HOME").unwrap_or_else(|_| "~".into())
    } else if word.starts_with("~/") {
        if let Ok(home) = env::var("HOME") {
            format!("{}{}", home, &word[1..])
        } else {
            word.to_string()
        }
    } else {
        word.to_string()
    }
}

pub fn expand_env_vars(word: &str) -> String {
    if !word.contains('$') {
        return word.to_string();
    }

    let mut result = String::new();
    let mut chars = word.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var_name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if !var_name.is_empty() {
                if let Ok(val) = env::var(&var_name) {
                    result.push_str(&val);
                }
            } else {
                result.push('$');
            }
        } else {
            result.push(ch);
        }
    }

    result
}