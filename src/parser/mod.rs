pub mod ast;
pub mod lexer;

use ast::{ChainOp, Command, Pipeline, Redirection};
use lexer::{tokenize, Token};

pub fn is_incomplete(input: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if !in_single && !in_double && ch == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if ch == '\\' && !in_single {
            escaped = true;
            i += 1;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        }
        i += 1;
    }

    if in_single || in_double || escaped {
        return true;
    }

    let tokens = match tokenize(input, 0) {
        Ok(tokens) => tokens,
        Err(error) => return error.contains("unmatched parameter expansion"),
    };
    matches!(
        tokens.last(),
        Some(
            Token::Pipe
                | Token::And
                | Token::Or
                | Token::RedirectOut
                | Token::RedirectAppend
                | Token::RedirectIn
                | Token::RedirectErr
                | Token::RedirectErrAppend
                | Token::RedirectAll
        )
    )
}

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
            if c == '#'
                && current
                    .chars()
                    .last()
                    .is_none_or(|previous| previous.is_whitespace())
            {
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

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
            } else if c == '\n' {
                let after_conditional = current.trim().is_empty()
                    && segments
                        .last()
                        .is_some_and(|(_, op)| matches!(op, Some(ChainOp::And | ChainOp::Or)));
                let after_redirection_or_pipe = current
                    .trim_end()
                    .chars()
                    .last()
                    .is_some_and(|previous| "|><".contains(previous));
                if after_conditional || after_redirection_or_pipe {
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    i += 1;
                    continue;
                }
                if !current.trim().is_empty() {
                    segments.push((current.trim().to_string(), Some(ChainOp::Sequence)));
                }
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
    let tokens = tokenize(input, last_status)?;
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
                    current_redirs.push(Redirection::OutputTruncate(
                        crate::expander::restore_quoted_chars(&filename),
                    ));
                } else {
                    return Err("bnb: syntax error near unexpected token '>'".into());
                }
            }
            Token::RedirectAppend => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::OutputAppend(
                        crate::expander::restore_quoted_chars(&filename),
                    ));
                } else {
                    return Err("bnb: syntax error near unexpected token '>>'".into());
                }
            }
            Token::RedirectIn => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::Input(crate::expander::restore_quoted_chars(
                        &filename,
                    )));
                } else {
                    return Err("bnb: syntax error near unexpected token '<'".into());
                }
            }
            Token::RedirectErr => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::StderrTruncate(
                        crate::expander::restore_quoted_chars(&filename),
                    ));
                } else {
                    return Err("bnb: syntax error near unexpected token '2>'".into());
                }
            }
            Token::RedirectErrAppend => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::StderrAppend(
                        crate::expander::restore_quoted_chars(&filename),
                    ));
                } else {
                    return Err("bnb: syntax error near unexpected token '2>>'".into());
                }
            }
            Token::RedirectAll => {
                if let Some(Token::Word(filename)) = iter.next() {
                    current_redirs.push(Redirection::OutputAndStderr(
                        crate::expander::restore_quoted_chars(&filename),
                    ));
                } else {
                    return Err("bnb: syntax error near unexpected token '&>'".into());
                }
            }
            Token::Background => {
                background = true;
                if iter.peek().is_some() {
                    return Err("bnb: syntax error: '&' must end a command".into());
                }
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
            Token::And | Token::Or | Token::Semicolon => {
                return Err("bnb: syntax error: unexpected command separator".into());
            }
        }
    }

    if !current_args.is_empty() {
        commands.push(Command {
            args: current_args,
            redirections: current_redirs,
        });
    } else if !commands.is_empty() {
        return Err("bnb: syntax error near unexpected token '|'".into());
    } else if !current_redirs.is_empty() {
        return Err("bnb: syntax error: redirection without a command".into());
    } else {
        return Err("bnb: syntax error: expected a command".into());
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

    #[test]
    fn test_newlines_separate_commands_outside_quotes() {
        let segs = split_chains("echo \"a\nb\"\necho c");
        assert_eq!(segs.len(), 2);
        assert_eq!(
            segs[0],
            ("echo \"a\nb\"".to_string(), Some(ChainOp::Sequence))
        );
        assert_eq!(segs[1], ("echo c".to_string(), None));
    }

    #[test]
    fn test_newline_after_pipe_or_conditional_operator_continues_command() {
        assert_eq!(
            split_chains("echo first |\ncat"),
            vec![("echo first | cat".to_string(), None)]
        );
        let conditional = split_chains("echo first &&\necho second");
        assert_eq!(conditional.len(), 2);
        assert_eq!(
            conditional[0],
            ("echo first".to_string(), Some(ChainOp::And))
        );
        assert_eq!(conditional[1], ("echo second".to_string(), None));
    }

    #[test]
    fn test_malformed_pipeline_syntax_is_rejected() {
        assert!(parse_pipeline("echo hi |", 0).is_err());
        assert!(parse_pipeline("> output.txt", 0).is_err());
        assert!(parse_pipeline("echo hi & echo bye", 0).is_err());
        assert!(parse_pipeline("echo hi && echo bye", 0).is_err());
    }

    #[test]
    fn test_incomplete_input_detection() {
        assert!(is_incomplete("echo 'open"));
        assert!(is_incomplete("echo value |"));
        assert!(is_incomplete("echo value &&"));
        assert!(is_incomplete("echo ${HOME"));
        assert!(!is_incomplete("echo 'closed'"));
        assert!(!is_incomplete("echo \"text>\""));
        assert!(!is_incomplete("echo # '"));
        assert!(!is_incomplete("echo value"));
    }

    #[test]
    fn test_comment_after_separator_is_ignored() {
        let segs = split_chains("echo first; # ignored\n echo second");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0], ("echo first".to_string(), Some(ChainOp::Sequence)));
        assert_eq!(segs[1], ("echo second".to_string(), None));
    }
}
