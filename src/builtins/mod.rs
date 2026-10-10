pub mod alias;
pub mod cd;
pub mod clear;
pub mod echo;
pub mod exit;
pub mod export;
pub mod history;
pub mod prompt_config;
pub mod pwd;
pub mod rm;
pub mod source;
pub mod unalias;
pub mod undo;
pub mod unset;
pub mod which;
pub mod z;

use std::io::Write;

pub fn is_builtin(cmd: &str) -> bool {
    matches!(
        cmd,
        "cd" | "mkcd"
            | "pwd"
            | "clear"
            | "echo"
            | "export"
            | "unset"
            | "alias"
            | "unalias"
            | "history"
            | "source"
            | "."
            | "which"
            | "type"
            | "undo"
            | "rm"
            | "z"
            | "prompt-config"
            | "exit"
            | "bnb-update"
    )
}

pub fn execute_with_writer<W: Write + ?Sized>(
    cmd: &str,
    args: &[String],
    _writer: &mut W,
) -> Result<(), String> {
    match cmd {
        "cd" => cd::run(args),
        "mkcd" => {
            if args.is_empty() {
                return Err("mkcd: missing directory argument".to_string());
            }
            std::fs::create_dir_all(&args[0])
                .map_err(|e| format!("mkcd: {}: {}", args[0], e))?;
            cd::run(args)
        }
        "pwd" => pwd::run(args),
        "clear" => clear::run(args),
        "echo" => echo::run(args),
        "export" => export::run(args),
        "unset" => unset::run(args),
        "alias" => alias::run(args),
        "unalias" => unalias::run(args),
        "history" => history::run(args),
        "source" | "." => source::run(args),
        "which" | "type" => which::run(args),
        "undo" => undo::run(args),
        "rm" => rm::run(args),
        "z" => z::run(args),
        "prompt-config" => prompt_config::run(),
        "exit" => exit::run(args),
        "bnb-update" => run_bnb_update(),
        _ => Err(format!("bnb: unknown builtin: {}", cmd)),
    }
}

#[allow(dead_code)]
pub fn dispatch(cmd: &str, args: &[String]) -> Option<Result<(), String>> {
    if is_builtin(cmd) {
        Some(execute_with_writer(cmd, args, &mut std::io::stdout()))
    } else {
        None
    }
}

fn run_bnb_update() -> Result<(), String> {
    println!("\x1b[1;36mChecking for bnb updates...\x1b[0m");
    let status = std::process::Command::new("cargo")
        .args(["install", "bnb-shell", "--force"])
        .status()
        .map_err(|e| format!("Failed to run cargo install: {}", e))?;

    if status.success() {
        println!("\x1b[1;32mSuccessfully updated bnb!\x1b[0m");
        Ok(())
    } else {
        Err("Failed to update bnb via cargo install.".to_string())
    }
}