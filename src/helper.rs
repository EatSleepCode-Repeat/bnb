use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Helper, Result};
use std::borrow::Cow;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

pub struct BnbHelper {
    pub completer: FilenameCompleter,
    pub hinter: HistoryHinter,
}

impl BnbHelper {
    pub fn new() -> Self {
        Self {
            completer: FilenameCompleter::new(),
            hinter: HistoryHinter::new(),
        }
    }
}

impl Completer for BnbHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_up_to_pos = &line[..pos];

        let word_start = line_up_to_pos
            .rfind(|c: char| c.is_whitespace() || "|&;><".contains(c))
            .map(|i| i + 1)
            .unwrap_or(0);

        let current_word = &line_up_to_pos[word_start..];
        let command_prefix = line_up_to_pos[..word_start].trim_end();
        let is_command_pos = command_prefix.trim().is_empty()
            || command_prefix
                .chars()
                .last()
                .is_some_and(|c| "|&;".contains(c));

        let mut matches = Vec::new();

        if is_command_pos && !current_word.contains('/') {
            let builtins = [
                "cd",
                "export",
                "unset",
                "alias",
                "unalias",
                "which",
                "type",
                "source",
                ".",
                "history",
                "echo",
                "pwd",
                "clear",
                "exit",
                "z",
                "mkcd",
                "bnb-update",
                "undo",
            ];
            for b in builtins {
                if b.starts_with(current_word) {
                    matches.push(Pair {
                        display: b.to_string(),
                        replacement: b.to_string(),
                    });
                }
            }

            for alias in crate::builtins::alias::names() {
                if alias.starts_with(current_word) {
                    matches.push(Pair {
                        display: alias.clone(),
                        replacement: alias,
                    });
                }
            }

            if let Ok(path_var) = env::var("PATH") {
                let mut seen = HashSet::new();
                for dir in env::split_paths(&path_var) {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            if let Ok(name) = entry.file_name().into_string() {
                                if name.starts_with(current_word) && seen.insert(name.clone()) {
                                    matches.push(Pair {
                                        display: name.clone(),
                                        replacement: name,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let (file_start, file_matches) = self.completer.complete(line, pos, ctx)?;
        if matches.is_empty() {
            Ok((file_start, file_matches))
        } else {
            matches.extend(file_matches);
            Ok((word_start, matches))
        }
    }
}

#[derive(Debug, PartialEq)]
enum TokenType {
    Command,
    Operator,
    EnvVar,
    StringLiteral,
    Normal,
}

fn tokenize_for_highlight(line: &str) -> Vec<(String, TokenType)> {
    let mut res = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut is_first_word = true;

    while i < n {
        let c = chars[i];

        if c.is_whitespace() {
            let start = i;
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            res.push((chars[start..i].iter().collect(), TokenType::Normal));
            continue;
        }

        if c == '&' && i + 1 < n && chars[i + 1] == '&' {
            res.push(("&&".to_string(), TokenType::Operator));
            i += 2;
            is_first_word = true;
            continue;
        }

        if c == '|' && i + 1 < n && chars[i + 1] == '|' {
            res.push(("||".to_string(), TokenType::Operator));
            i += 2;
            is_first_word = true;
            continue;
        }

        if "|&;".contains(c) {
            res.push((c.to_string(), TokenType::Operator));
            i += 1;
            is_first_word = true;
            continue;
        }

        if "><".contains(c) {
            let start = i;
            if c == '>' && i + 1 < n && (chars[i + 1] == '>' || chars[i + 1] == '&') {
                i += 2;
            } else {
                i += 1;
            }
            res.push((chars[start..i].iter().collect(), TokenType::Operator));
            continue;
        }

        if c == '$' {
            let start = i;
            i += 1;
            if i < n && (chars[i] == '?' || chars[i] == '$') {
                i += 1;
            } else if i < n && chars[i] == '{' {
                while i < n && chars[i] != '}' {
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
            } else {
                while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
            }
            res.push((chars[start..i].iter().collect(), TokenType::EnvVar));
            is_first_word = false;
            continue;
        }

        if c == '"' || c == '\'' {
            let quote = c;
            let start = i;
            i += 1;
            while i < n && chars[i] != quote {
                i += 1;
            }
            if i < n {
                i += 1;
            }
            res.push((chars[start..i].iter().collect(), TokenType::StringLiteral));
            is_first_word = false;
            continue;
        }

        let start = i;
        while i < n && !chars[i].is_whitespace() && !"|&;><\"'$".contains(chars[i]) {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();

        if is_first_word {
            res.push((word, TokenType::Command));
            is_first_word = false;
        } else {
            res.push((word, TokenType::Normal));
        }
    }

    res
}

fn is_valid_cmd(cmd: &str) -> bool {
    if crate::builtins::is_builtin(cmd) {
        return true;
    }
    if crate::builtins::alias::resolve(cmd).is_some() {
        return true;
    }
    if cmd.contains('/') {
        return Path::new(cmd).is_file();
    }

    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            let clean = dir.trim_matches('"').trim_matches('\'');
            if PathBuf::from(clean).join(cmd).is_file() {
                return true;
            }
        }
    }

    let default_dirs = [
        "/bin",
        "/usr/bin",
        "/usr/local/bin",
        "/opt/homebrew/bin",
        "/usr/sbin",
        "/sbin",
    ];
    for dir in default_dirs {
        if PathBuf::from(dir).join(cmd).is_file() {
            return true;
        }
    }

    false
}

impl Highlighter for BnbHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.trim().is_empty() {
            return Cow::Borrowed(line);
        }

        let tokens = tokenize_for_highlight(line);
        let mut highlighted = String::new();

        for (text, token_type) in tokens {
            match token_type {
                TokenType::Command => {
                    if is_valid_cmd(&text) {
                        highlighted.push_str("\x1b[1;32m");
                    } else {
                        highlighted.push_str("\x1b[1;31m");
                    }
                    highlighted.push_str(&text);
                    highlighted.push_str("\x1b[0m");
                }
                TokenType::Operator => {
                    highlighted.push_str("\x1b[1;33m");
                    highlighted.push_str(&text);
                    highlighted.push_str("\x1b[0m");
                }
                TokenType::EnvVar => {
                    highlighted.push_str("\x1b[1;35m");
                    highlighted.push_str(&text);
                    highlighted.push_str("\x1b[0m");
                }
                TokenType::StringLiteral => {
                    highlighted.push_str("\x1b[36m");
                    highlighted.push_str(&text);
                    highlighted.push_str("\x1b[0m");
                }
                TokenType::Normal => {
                    highlighted.push_str(&text);
                }
            }
        }

        Cow::Owned(highlighted)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

impl Hinter for BnbHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<Self::Hint> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Validator for BnbHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult> {
        if crate::parser::is_incomplete(ctx.input()) {
            Ok(ValidationResult::Incomplete)
        } else {
            Ok(ValidationResult::Valid(None))
        }
    }
}
impl Helper for BnbHelper {}
