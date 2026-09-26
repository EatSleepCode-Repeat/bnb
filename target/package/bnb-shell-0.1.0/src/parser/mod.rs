pub mod ast;
pub mod lexer;

use ast::{Command, Pipeline, Redirection};
use lexer::{tokenize, Token};

pub fn parse(input: &str) -> Result<Pipeline, String> {
    let tokens = tokenize(input);
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
            Token::Pipe => {
                if current_args.is_empty() {
                    return Err("bnb: syntax error near unexpected token '|'".into());
                }
                commands.push(Command {
                    args: current_args,
                    redirections: current_redirs,
                });
                current_args = Vec::new();
                current_redirs = Vec::new();
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
            Token::Background => {
                background = true;
            }
            Token::Word(word) => {
                current_args.push(word);
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
    }

    Ok(Pipeline {
        commands,
        run_in_background: background,
    })
}