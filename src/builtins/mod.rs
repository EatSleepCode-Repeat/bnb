pub mod alias;
pub mod cd;
pub mod exit;
pub mod export;
pub mod z;

use std::fs;
use std::path::PathBuf;

pub fn is_builtin(cmd: &str) -> bool {
    matches!(
        cmd,
        "cd" | "export" | "alias" | "exit" | "z" | "mkcd" | "bnb-update"
    )
}

pub fn execute(cmd: &str, args: &[String]) -> Result<(), String> {
    match cmd {
        "cd" => cd::run(args),
        "export" => export::run(args),
        "alias" => alias::run(args),
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

    cd::run(&[target.clone()])
}
