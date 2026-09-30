pub mod ast;
pub mod lexer;

use ast::{ChainOp, Command, Pipeline, Redirection};
use lexer::{tokenize, Token};

pub fn split_chains(input: &str) -> Vec<(String, Option<ChainOp>)> {
    let mut segments = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    while i < n {
        let c = chars[i];

        if c == '\'' && !in_double {
            in_single = !in_single;
            current.push(c);
            i += 1;
            continue;
        }

        if c == '"' && !in_single {
            in_double = !in_double;
            current.push(c);
            i += 1;
            continue;
        }

        if c == '\\' && (in_double || !in_single) {
            current.push(c);
            i += 1;
            if i < n {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        if !in_single && !in_double {
            if c == '&' && i + 1 < n && chars[i + 1] == '&' {
                segments.push((current.trim().to_string(), Some(ChainOp::And)));
                current.clear();
                i += 2;
                continue;
            } else if c == '|' && i + 1 < n && chars[i + 1] == '|' {
                segments.push((current.trim().to_string(), Some(ChainOp::Or)));
                current.clear();
                i += 2;
                continue;
            } else if c == ';' {
                segments.push((current.trim().to_string(), Some(ChainOp::Sequence)));
                current.clear();
                i += 1;
                continue;
            }
        }

        current.push(c);
        i += 1;
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push((trimmed.to_string(), None));
    }

    segments
}

pub fn parse_pipeline(input: &str, last_status: i32) -> Result<Pipeline, String> {
    let tokens = tokenize(input, last_status);
    if tokens.is_empty() {
        return Err("Empty command".into());
    }

    let mut commands = Vec::new();
    let mut current_args = Vec::new();
    let mut current_redirs = Vec::new();
    let mut background = false;

    let mut iter = tokens.into_iter().peekable();

    while let Some(token) = iter.next() {
        match token {
            Token::Word(w) => {
                current_args.push(w);
            }
            Token::RedirectOut => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::OutputTruncate(filename));
                } else {
                    return Err("bnb: syntax error near unexpected token '>'".into());
                }
            }
            Token::RedirectAppend => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::OutputAppend(filename));
                } else {
                    return Err("bnb: syntax error near unexpected token '>>'".into());
                }
            }
            Token::RedirectIn => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::Input(filename));
                } else {
                    return Err("bnb: syntax error near unexpected token '<'".into());
                }
            }
            Token::RedirectErr => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::StderrTruncate(filename));
                } else {
                    return Err("bnb: syntax error near unexpected token '2>'".into());
                }
            }
            Token::RedirectErrAppend => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::StderrAppend(filename));
                } else {
                    return Err("bnb: syntax error near unexpected token '2>>'".into());
                }
            }
            Token::RedirectAll => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::OutputAndStderr(filename));
                } else {
                    return Err("bnb: syntax error near unexpected token '&>'".into());
                }
            }
            Token::Background => {
                background = true;
            }
            Token::Pipe => {
                if current_args.is_empty() {
                    return Err("bnb: syntax error near unexpected token '|'".into());
                }
                commands.push(Command {
                    args: std::mem::take(&mut current_args),
                    redirections: std::mem::take(&mut current_redirs),
                });
            }
            _ => {}
        }
    }

    if !current_args.is_empty() {
        commands.push(Command {
            args: current_args,
            redirections: current_redirs,
        });
    } else if !commands.is_empty() {
        return Err("bnb: syntax error near unexpected token '|'".into());
    }

    Ok(Pipeline {
        commands,
        run_in_background: background,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_chains() {
        let segs = split_chains("echo a && echo b || echo c ; echo d");
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0], ("echo a".to_string(), Some(ChainOp::And)));
        assert_eq!(segs[1], ("echo b".to_string(), Some(ChainOp::Or)));
        assert_eq!(segs[2], ("echo c".to_string(), Some(ChainOp::Sequence)));
        assert_eq!(segs[3], ("echo d".to_string(), None));
    }

    #[test]
    fn test_split_chains_with_quotes() {
        let segs = split_chains("echo \"a && b\" || echo 'c ; d'");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0], ("echo \"a && b\"".to_string(), Some(ChainOp::Or)));
        assert_eq!(segs[1], ("echo 'c ; d'".to_string(), None));
    }
}