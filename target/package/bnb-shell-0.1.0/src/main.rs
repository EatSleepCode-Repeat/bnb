mod builtins;
mod config;
mod executor;
mod helper;
mod parser;
mod prompt;

use std::env;
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

fn main() {
    ensure_system_path();
    config::load_config();
    ensure_system_path();

    if let Ok(pwd) = env::current_dir() {
        builtins::z::add_path(&pwd);
    }

    let config = Config::builder().build();
    let mut rl: Editor<BnbHelper, DefaultHistory> =
        Editor::with_config(config).expect("Failed to initialize line reader");

    rl.set_helper(Some(BnbHelper::new()));

    let history_file = env::var("HOME")
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

                match parser::parse(trimmed) {
                    Ok(mut pipeline) => {
                        if pipeline.commands.len() == 1 && !pipeline.commands[0].args.is_empty() {
                            let first_word = &pipeline.commands[0].args[0];
                            if let Some(aliased) = builtins::alias::resolve(first_word) {
                                let extra_args = pipeline.commands[0].args[1..].to_vec();
                                if let Ok(mut aliased_pipeline) = parser::parse(&aliased) {
                                    if !aliased_pipeline.commands.is_empty() {
                                        aliased_pipeline.commands[0].args.extend(extra_args);
                                        pipeline = aliased_pipeline;
                                    }
                                }
                            }
                        }

                        if pipeline.commands.len() == 1 {
                            let cmd = &pipeline.commands[0];
                            if !cmd.args.is_empty() && builtins::is_builtin(&cmd.args[0]) {
                                if let Err(e) = builtins::execute(&cmd.args[0], &cmd.args[1..]) {
                                    eprintln!("{}", e);
                                    last_status = Some(1);
                                } else {
                                    last_status = Some(0);
                                }
                                last_duration = Some(start_time.elapsed());
                                continue;
                            }
                        }

                        match executor::process::run_pipeline(&pipeline) {
                            Ok(code) => {
                                last_status = Some(code);
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                last_status = Some(127);
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("{}", err);
                        last_status = Some(2);
                    }
                }

                last_duration = Some(start_time.elapsed());

                let _ = io::stdout().flush();
                let _ = io::stderr().flush();
            }
            Err(ReadlineError::Interrupted) => {
                last_duration = None;
                last_status = None;
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
