use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

fn get_history_file() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".bnb_history"))
}

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    let path = match get_history_file() {
        Some(p) => p,
        None => return Err("history: could not determine history file path".to_string()),
    };

    if args.iter().any(|a| a == "-c") {
        let _ = std::fs::write(&path, "");
        let _ = writeln!(out, "bnb: history cleared");
        return Ok(());
    }

    if let Ok(file) = File::open(&path) {
        let reader = BufReader::new(file);
        for (i, line) in reader.lines().flatten().enumerate() {
            let _ = writeln!(out, "  \x1b[1;36m{:4}\x1b[0m  {}", i + 1, line);
        }
    }

    Ok(())
}
