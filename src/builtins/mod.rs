pub mod alias;
pub mod cd;
pub mod clear;
pub mod echo;
pub mod exit;
pub mod export;
pub mod history;
pub mod pwd;
pub mod source;
pub mod unalias;
pub mod unset;
pub mod which;
pub mod z;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn is_builtin(cmd: &str) -> bool {
    matches!(
        cmd,
        "cd" | "export"
            | "unset"
            | "alias"
            | "unalias"
            | "which"
            | "type"
            | "source"
            | "."
            | "history"
            | "echo"
            | "pwd"
            | "clear"
            | "exit"
            | "z"
            | "mkcd"
            | "bnb-update"
    )
}

#[allow(dead_code)]
pub fn execute(cmd: &str, args: &[String]) -> Result<(), String> {
    execute_with_writer(cmd, args, &mut std::io::stdout())
}

pub fn execute_with_writer(cmd: &str, args: &[String], out: &mut dyn Write) -> Result<(), String> {
    match cmd {
        "cd" => cd::run(args),
        "export" => export::run_with_writer(args, out),
        "unset" => unset::run(args),
        "alias" => alias::run_with_writer(args, out),
        "unalias" => unalias::run(args),
        "which" | "type" => which::run_with_writer(args, out),
        "source" | "." => source::run(args),
        "history" => history::run_with_writer(args, out),
        "echo" => echo::run_with_writer(args, out),
        "pwd" => pwd::run_with_writer(args, out),
        "clear" => clear::run_with_writer(args, out),
        "exit" => exit::run(args),
        "z" => z::run(args),
        "mkcd" => run_mkcd(args),
        "bnb-update" => crate::updater::run_update(),
        _ => Err(format!("bnb: unknown builtin: {}", cmd)),
    }
}

fn run_mkcd(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("mkcd: missing directory operand".to_string());
    }

    let expanded = crate::expander::expand_args(args);
    let target = &expanded[0];
    let path = PathBuf::from(target);

    fs::create_dir_all(&path)
        .map_err(|e| format!("mkcd: failed to create {}: {}", target, e))?;

    cd::run(std::slice::from_ref(target))
}
