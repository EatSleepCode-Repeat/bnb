mod builtins;
mod config;
mod executor;
mod expander;
mod helper;
mod parser;
mod prompt;
mod updater;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

use helper::BnbHelper;
use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

fn ensure_system_path() {
    let standard_paths = [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ];

    let mut current_paths: Vec<String> = match env::var("PATH") {
        Ok(p) => p.split(':').map(|s| s.to_string()).collect(),
        Err(_) => Vec::new(),
    };

    if let Ok(home) = env::var("HOME") {
        let cargo_bin = format!("{}/.cargo/bin", home);
        if !current_paths.contains(&cargo_bin) {
            current_paths.insert(0, cargo_bin);
        }
    }

    for path in standard_paths.iter().rev() {
        let s = path.to_string();
        if !current_paths.contains(&s) {
            current_paths.insert(0, s);
        }
    }

    env::set_var("PATH", current_paths.join(":"));
}

fn print_help() {
    println!("\x1b[1;36mbnb-shell\x1b[0m v{}", updater::CURRENT_VERSION);
    println!("A fast, cross-platform terminal shell built in Rust with Powerlevel10k aesthetics.\n");
    println!("\x1b[1mUSAGE:\x1b[0m");
    println!("    bnb [FLAGS] [SCRIPT_FILE]");
    println!("    bnb -c <COMMAND>\n");
    println!("\x1b[1mFLAGS:\x1b[0m");
    println!("    -c <CMD>         Execute command string non-interactively and exit");
    println!("    -h, --help       Print this help message and exit");
    println!("    -V, --version    Print version information and exit\n");
    println!("\x1b[1mCONFIG:\x1b[0m");
    println!("    ~/.bnbrc         Loaded on startup (see .bnbrc.example)\n");
    println!("\x1b[1mBUILTINS:\x1b[0m");
    println!("    alias, cd, clear, echo, exit, export, history, mkcd,");
    println!("    pwd, source (.), type, unalias, unset, which, z, bnb-update");
    println!("    Run any builtin with no args or --help for usage info.");
}

pub fn run_line(line: &str, mut last_status: i32) -> i32 {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return last_status;
    }

    let segments = parser::split_chains(trimmed);
    let mut should_run = true;

    for (seg_str, next_op) in segments {
        if seg_str.is_empty() {
            continue;
        }

        if should_run {
            match parser::parse_pipeline(&seg_str, last_status) {
                Ok(pipeline) => match executor::process::run_pipeline(&pipeline) {
                    Ok(code) => {
                        last_status = code;
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        last_status = 127;
                    }
                },
                Err(err) => {
                    eprintln!("{}", err);
                    last_status = 2;
                }
            }
        }

        match next_op {
            Some(parser::ast::ChainOp::And) => {
                should_run = last_status == 0;
            }
            Some(parser::ast::ChainOp::Or) => {
                should_run = last_status != 0;
            }
            Some(parser::ast::ChainOp::Sequence) => {
                should_run = true;
            }
            None => break,
        }
    }

    last_status
}

fn run_script_file(path_str: &str) -> i32 {
    let path = PathBuf::from(path_str);
    match fs::read_to_string(&path) {
        Ok(content) => {
            let mut status = 0;
            for line in content.lines() {
                status = run_line(line, status);
            }
            status
        }
        Err(e) => {
            eprintln!("bnb: cannot read {}: {}", path_str, e);
            1
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        updater::print_version();
        return;
    }

    // Handle -c <command>
    if let Some(pos) = args.iter().position(|a| a == "-c") {
        if pos + 1 < args.len() {
            let cmd_str = &args[pos + 1];
            ensure_system_path();
            config::load_config();
            ensure_system_path();
            let code = run_line(cmd_str, 0);
            std::process::exit(code);
        } else {
            eprintln!("bnb: -c requires a command string");
            std::process::exit(2);
        }
    }

    // Handle script file argument: bnb myscript.bnb
    let non_flags: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    if let Some(&script_path) = non_flags.first() {
        if PathBuf::from(script_path).is_file() {
            ensure_system_path();
            config::load_config();
            ensure_system_path();
            let code = run_script_file(script_path);
            std::process::exit(code);
        }
    }

    ensure_system_path();
    config::load_config();
    ensure_system_path();

    updater::check_for_updates_async();
    updater::print_update_banner_if_available();

    if let Ok(pwd) = env::current_dir() {
        builtins::z::add_path(&pwd);
    }

    let config = Config::builder().build();
    let mut rl: Editor<BnbHelper, DefaultHistory> =
        Editor::with_config(config).expect("Failed to initialize line reader");

    rl.set_helper(Some(BnbHelper::new()));

    let history_file = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".bnb_history"))
        .ok();

    if let Some(ref path) = history_file {
        let _ = rl.load_history(path);
    }

    let mut last_duration = None;
    let mut last_status = None;

    loop {
        let prompt_str = prompt::get_prompt(last_duration, last_status);

        match rl.readline(&prompt_str) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    last_duration = None;
                    continue;
                }

                let _ = rl.add_history_entry(trimmed);
                let start_time = Instant::now();

                let code = run_line(trimmed, last_status.unwrap_or(0));
                last_status = Some(code);
                last_duration = Some(start_time.elapsed());

                let _ = io::stdout().flush();
                let _ = io::stderr().flush();
            }
            Err(ReadlineError::Interrupted) => {
                last_duration = None;
                last_status = Some(130);
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("bnb error: {:?}", err);
                break;
            }
        }
    }

    if let Some(ref path) = history_file {
        let _ = rl.save_history(path);
    }
}
