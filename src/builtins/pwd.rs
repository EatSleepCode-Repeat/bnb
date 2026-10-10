use std::env;
use std::io::Write;

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(_args: &[String], out: &mut dyn Write) -> Result<(), String> {
    let current =
        env::current_dir().map_err(|e| format!("pwd: error getting current directory: {}", e))?;
    let _ = writeln!(out, "{}", current.display());
    Ok(())
}
