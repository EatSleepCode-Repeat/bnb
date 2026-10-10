mod builtins;
mod config;
mod executor;
mod expander;
mod helper;
mod parser;
mod prompt;
mod updater;

pub mod safety;
pub mod tui;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

use helper::BnbHelper;
use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{
    Cmd, ConditionalEventHandler, Editor, Event, EventContext, EventHandler, KeyCode, KeyEvent,
    Modifiers, Movement,
};

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

fn load_startup_config() {
    if let Err(error) = config::load_config() {
        eprintln!("bnb: config: {}", error);
    }
}

fn print_help() {
    println!("\x1b[1;36mbnb-shell\x1b[0m v{}", updater::CURRENT_VERSION);
    println!("A zsh-inspired Rust shell with explicit syntax and config support.\n");

    println!("\x1b[1;33mUSAGE:\x1b[0m");
    println!("    bnb [FLAGS] [SCRIPT_FILE]");
    println!("    bnb -c <COMMAND>\n");

    println!("\x1b[1;33mFLAGS:\x1b[0m");
    println!(
        "    \x1b[36m-c <CMD>\x1b[0m         Execute command string non-interactively and exit"
    );
    println!("    \x1b[36m-h, --help\x1b[0m       Print this help message and exit");
    println!("    \x1b[36m-V, --version\x1b[0m    Print version information and exit\n");

    println!("\x1b[1;33mKEYBINDINGS & TUIs:\x1b[0m");
    println!("    \x1b[36mCtrl+K\x1b[0m           Universal Command Palette");
    println!("    \x1b[36mCtrl+R\x1b[0m           Interactive TUI fuzzy history finder");
    println!("    \x1b[36mCtrl+F\x1b[0m           Interactive TUI fuzzy file finder");
    println!("    \x1b[36mCtrl+B\x1b[0m           Interactive TUI Git branch switcher");
    println!("    \x1b[36mCtrl+Z\x1b[0m           Interactive TUI frecency directory jumper");
    println!("    \x1b[36mCtrl+O\x1b[0m           Interactive TUI global project switcher");
    println!("    \x1b[36mCtrl+G\x1b[0m           Interactive TUI Git status stager");
    println!("    \x1b[36mCtrl+T\x1b[0m           Interactive prompt theme wizard\n");

    println!("\x1b[1;33mCONFIG:\x1b[0m");
    println!("    \x1b[36m~/.bnbrc\x1b[0m         Loaded on startup (see .bnbrc.example)\n");

    println!("\x1b[1;33mBUILTINS:\x1b[0m");
    println!("    alias, alias-config, cd, clear, cyberpunk, echo, exit, export, history, mkcd,");
    println!(
        "    prompt-config, pwd, source (.), type, unalias, undo, unset, which, z, bnb-update\n"
    );
}

pub fn run_line(line: &str, mut last_status: i32) -> i32 {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return last_status;
    }

    let segments = parser::split_chains(trimmed);
    if segments.is_empty()
        || segments.iter().any(|(segment, _)| segment.is_empty())
        || segments.last().is_some_and(|(_, next_op)| {
            matches!(
                next_op,
                Some(parser::ast::ChainOp::And | parser::ast::ChainOp::Or)
            )
        })
    {
        eprintln!("bnb: syntax error: incomplete command chain");
        return 2;
    }

    let mut should_run = true;

    for (seg_str, next_op) in segments {
        if should_run {
            match parser::parse_pipeline(&seg_str, last_status) {
                Ok(pipeline) => match executor::process::run_pipeline(&pipeline) {
                    Ok(code) => {
                        last_status = code;
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        if e.starts_with("bnb: command not found:") {
                            if let Some(corrected_line) = suggest_and_prompt_typo(trimmed, &e) {
                                return run_line(&corrected_line, last_status);
                            }
                            last_status = 127;
                        } else {
                            last_status = 1;
                        }
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

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len_a + 1) {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate() {
        *cell = j;
    }

    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[len_a][len_b]
}

fn get_command_candidates() -> Vec<String> {
    let mut candidates = vec![
        "cd".into(),
        "mkcd".into(),
        "pwd".into(),
        "clear".into(),
        "echo".into(),
        "export".into(),
        "unset".into(),
        "alias".into(),
        "alias-config".into(),
        "unalias".into(),
        "history".into(),
        "source".into(),
        "which".into(),
        "type".into(),
        "undo".into(),
        "z".into(),
        "prompt-config".into(),
        "cyberpunk".into(),
        "exit".into(),
        "bnb-update".into(),
        "git".into(),
        "cargo".into(),
        "rustc".into(),
        "code".into(),
        "vim".into(),
        "nvim".into(),
        "docker".into(),
        "npm".into(),
        "node".into(),
        "python".into(),
        "python3".into(),
        "make".into(),
        "grep".into(),
        "find".into(),
        "curl".into(),
        "wget".into(),
        "ssh".into(),
        "brew".into(),
        "cat".into(),
        "ls".into(),
    ];

    if let Ok(path_var) = env::var("PATH") {
        for path_dir in path_var.split(':') {
            if let Ok(entries) = fs::read_dir(path_dir) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if !ft.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if !candidates.contains(&name) {
                                candidates.push(name);
                            }
                        }
                    }
                }
            }
        }
    }

    candidates
}

fn suggest_and_prompt_typo(trimmed_line: &str, err_msg: &str) -> Option<String> {
    let failed_cmd = err_msg.strip_prefix("bnb: command not found: ")?.trim();
    if failed_cmd.is_empty() {
        return None;
    }

    let candidates = get_command_candidates();
    let mut best_candidate: Option<String> = None;
    let mut best_dist = usize::MAX;

    for candidate in candidates {
        let dist = levenshtein_distance(failed_cmd, &candidate);
        if dist < best_dist && dist <= 2 && dist < failed_cmd.len() {
            best_dist = dist;
            best_candidate = Some(candidate);
        }
    }

    let suggested_cmd = best_candidate?;
    let mut parts: Vec<&str> = trimmed_line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    parts[0] = &suggested_cmd;
    let corrected_line = parts.join(" ");

    eprintln!(
        "\x1b[1;33m💡 Did you mean '\x1b[1;36m{}\x1b[1;33m'? Press \x1b[1;32m[Enter]\x1b[1;33m to run '\x1b[1;37m{}\x1b[1;33m', or \x1b[1;31m[Esc]\x1b[1;33m to cancel.\x1b[0m",
        suggested_cmd, corrected_line
    );

    let mut is_accepted = false;
    let _ = crossterm::terminal::enable_raw_mode();
    loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char('y') => {
                    is_accepted = true;
                    break;
                }
                crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('n') => {
                    break;
                }
                _ => {}
            }
        }
    }
    let _ = crossterm::terminal::disable_raw_mode();
    eprintln!();

    if is_accepted {
        Some(corrected_line)
    } else {
        None
    }
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

struct FuzzyHistoryHandler;

impl ConditionalEventHandler for FuzzyHistoryHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let history = prompt::load_history_entries();
        match tui::fuzzy::FuzzyFinder::select("History Search", &history) {
            Ok(Some(selected)) => Some(Cmd::Replace(Movement::BeginningOfLine, Some(selected))),
            Ok(None) => Some(Cmd::Noop),
            Err(error) => {
                eprintln!("{}", error);
                Some(Cmd::Noop)
            }
        }
    }
}

struct FuzzyFileHandler;

impl ConditionalEventHandler for FuzzyFileHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let files = tui::fuzzy::collect_files(10_000);
        match tui::fuzzy::FuzzyFinder::select("File Finder", &files) {
            Ok(Some(selected)) => Some(Cmd::Replace(Movement::ForwardChar(0), Some(selected))),
            Ok(None) => Some(Cmd::Noop),
            Err(error) => {
                eprintln!("{}", error);
                Some(Cmd::Noop)
            }
        }
    }
}

struct FuzzyGitBranchHandler;

impl ConditionalEventHandler for FuzzyGitBranchHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let branches = tui::fuzzy::collect_git_branches();
        if branches.is_empty() {
            eprintln!("\r\nbnb: not a git repository (or no branches found)");
            return Some(Cmd::Noop);
        }

        match tui::fuzzy::FuzzyFinder::select("Git Branch Switcher", &branches) {
            Ok(Some(selected)) => {
                let branch_target = selected.strip_prefix("origin/").unwrap_or(&selected);
                Some(Cmd::Replace(
                    Movement::BeginningOfLine,
                    Some(format!("git checkout {}", branch_target)),
                ))
            }
            Ok(None) => Some(Cmd::Noop),
            Err(error) => {
                eprintln!("{}", error);
                Some(Cmd::Noop)
            }
        }
    }
}

struct FuzzyZDirectoryHandler;

impl ConditionalEventHandler for FuzzyZDirectoryHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let dirs = tui::fuzzy::collect_z_directories();
        if dirs.is_empty() {
            eprintln!("\r\nbnb: no frecency directory history available yet");
            return Some(Cmd::Noop);
        }

        match tui::fuzzy::FuzzyFinder::select("Directory Jumper (z)", &dirs) {
            Ok(Some(selected)) => Some(Cmd::Replace(
                Movement::BeginningOfLine,
                Some(format!("cd \"{}\"", selected)),
            )),
            Ok(None) => Some(Cmd::Noop),
            Err(error) => {
                eprintln!("{}", error);
                Some(Cmd::Noop)
            }
        }
    }
}

struct FuzzyProjectHandler;

impl ConditionalEventHandler for FuzzyProjectHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let projects = tui::fuzzy::collect_projects();
        if projects.is_empty() {
            eprintln!("\r\nbnb: no git projects found under home directory");
            return Some(Cmd::Noop);
        }

        match tui::fuzzy::FuzzyFinder::select("Project Switcher", &projects) {
            Ok(Some(selected)) => {
                let expanded = crate::expander::expand_tilde(&selected);
                Some(Cmd::Replace(
                    Movement::BeginningOfLine,
                    Some(format!("cd \"{}\"", expanded)),
                ))
            }
            Ok(None) => Some(Cmd::Noop),
            Err(error) => {
                eprintln!("{}", error);
                Some(Cmd::Noop)
            }
        }
    }
}

struct FuzzyGitStagerHandler;

impl ConditionalEventHandler for FuzzyGitStagerHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let items = tui::fuzzy::collect_git_status_items();
        if items.is_empty() {
            eprintln!("\r\nbnb: working tree clean (no modified or untracked files)");
            return Some(Cmd::Noop);
        }

        match tui::fuzzy::FuzzyFinder::select_multi("Git Status Stager", &items) {
            Ok(selections) if !selections.is_empty() => {
                let mut to_stage = Vec::new();
                let mut to_unstage = Vec::new();

                for item in selections {
                    if let Some(file) = item.strip_prefix("[Staged] ") {
                        to_unstage.push(format!("\"{}\"", file));
                    } else if let Some(file) = item.strip_prefix("[Unstaged] ") {
                        to_stage.push(format!("\"{}\"", file));
                    } else if let Some(file) = item.strip_prefix("[Untracked] ") {
                        to_stage.push(format!("\"{}\"", file));
                    }
                }

                let mut commands = Vec::new();
                if !to_stage.is_empty() {
                    commands.push(format!("git add {}", to_stage.join(" ")));
                }
                if !to_unstage.is_empty() {
                    commands.push(format!("git restore --staged {}", to_unstage.join(" ")));
                }

                if !commands.is_empty() {
                    Some(Cmd::Replace(
                        Movement::BeginningOfLine,
                        Some(commands.join(" && ")),
                    ))
                } else {
                    Some(Cmd::Noop)
                }
            }
            _ => Some(Cmd::Noop),
        }
    }
}

struct FuzzyPromptThemeHandler;

impl ConditionalEventHandler for FuzzyPromptThemeHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let _ = crate::tui::prompt_wizard::run_wizard();
        Some(Cmd::Noop)
    }
}

struct FuzzyCommandPaletteHandler;

impl ConditionalEventHandler for FuzzyCommandPaletteHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let palette = vec![
            "🔍 History Search (Ctrl+R)".to_string(),
            "📄 Fuzzy File Finder (Ctrl+F)".to_string(),
            "🌿 Git Branch Switcher (Ctrl+B)".to_string(),
            "📁 Directory Jumper (Ctrl+Z)".to_string(),
            "🚀 Global Project Switcher (Ctrl+O)".to_string(),
            "📝 Git Status Stager (Ctrl+G)".to_string(),
            "⚙️ Configure Prompt Theme (Ctrl+T)".to_string(),
            "⚡ Configure Aliases (alias-config)".to_string(),
            "🎆 Cyberpunk Fireworks (cyberpunk)".to_string(),
            "🗑️ Visual Trash Vault (Ctrl+U)".to_string(),
            "🔄 Reload Shell Config (source ~/.bnbrc)".to_string(),
            "🧹 Clear Terminal Screen".to_string(),
            "🚪 Exit Shell Session".to_string(),
        ];

        match tui::fuzzy::FuzzyFinder::select("Command Palette", &palette) {
            Ok(Some(selected)) => {
                if selected.contains("History Search") {
                    FuzzyHistoryHandler.handle(_evt, _n, _positive, _ctx)
                } else if selected.contains("Fuzzy File Finder") {
                    FuzzyFileHandler.handle(_evt, _n, _positive, _ctx)
                } else if selected.contains("Git Branch Switcher") {
                    FuzzyGitBranchHandler.handle(_evt, _n, _positive, _ctx)
                } else if selected.contains("Directory Jumper") {
                    FuzzyZDirectoryHandler.handle(_evt, _n, _positive, _ctx)
                } else if selected.contains("Global Project Switcher") {
                    FuzzyProjectHandler.handle(_evt, _n, _positive, _ctx)
                } else if selected.contains("Git Status Stager") {
                    FuzzyGitStagerHandler.handle(_evt, _n, _positive, _ctx)
                } else if selected.contains("Configure Prompt Theme") {
                    let _ = crate::tui::prompt_wizard::run_wizard();
                    Some(Cmd::Noop)
                } else if selected.contains("Configure Aliases") {
                    let _ = crate::tui::alias_manager::AliasManager::run();
                    Some(Cmd::Noop)
                } else if selected.contains("Cyberpunk Fireworks") {
                    let _ = crate::tui::fireworks::run_fireworks();
                    Some(Cmd::Noop)
                } else if selected.contains("Visual Trash Vault") {
                    let _ = crate::tui::trash_vault::run_trash_vault();
                    Some(Cmd::Noop)
                } else if selected.contains("Reload Shell Config") {
                    Some(Cmd::Replace(
                        Movement::BeginningOfLine,
                        Some("source ~/.bnbrc".to_string()),
                    ))
                } else if selected.contains("Clear Terminal Screen") {
                    Some(Cmd::ClearScreen)
                } else if selected.contains("Exit Shell Session") {
                    Some(Cmd::Replace(
                        Movement::BeginningOfLine,
                        Some("exit".to_string()),
                    ))
                } else {
                    Some(Cmd::Noop)
                }
            }
            _ => Some(Cmd::Noop),
        }
    }
}

struct FuzzyTrashVaultHandler;

impl ConditionalEventHandler for FuzzyTrashVaultHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: usize,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        let _ = crate::tui::trash_vault::run_trash_vault();
        Some(Cmd::Noop)
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

    if let Some(pos) = args.iter().position(|a| a == "-c") {
        if pos + 1 < args.len() {
            let cmd_str = &args[pos + 1];
            ensure_system_path();
            load_startup_config();
            ensure_system_path();
            let code = run_line(cmd_str, 0);
            std::process::exit(code);
        } else {
            eprintln!("bnb: -c requires a command string");
            std::process::exit(2);
        }
    }

    let non_flags: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    if let Some(&script_path) = non_flags.first() {
        if PathBuf::from(script_path).is_file() {
            ensure_system_path();
            load_startup_config();
            ensure_system_path();
            let code = run_script_file(script_path);
            std::process::exit(code);
        }
    }

    ensure_system_path();
    load_startup_config();
    ensure_system_path();

    builtins::cd::check_and_load_dotenv();

    updater::check_for_updates_async();
    updater::print_update_banner_if_available();

    if let Ok(pwd) = env::current_dir() {
        builtins::z::add_path(&pwd);
    }

    let config = Config::builder().build();
    let mut rl: Editor<BnbHelper, DefaultHistory> =
        Editor::with_config(config).expect("Failed to initialize line reader");

    rl.set_helper(Some(BnbHelper::new()));

    // Command Palette (Ctrl+K)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('k'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyCommandPaletteHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('K'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyCommandPaletteHandler)),
    );

    // History Search (Ctrl+R)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('r'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyHistoryHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('R'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyHistoryHandler)),
    );

    // File Finder (Ctrl+F)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('f'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyFileHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('F'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyFileHandler)),
    );

    // Git Branch Switcher (Ctrl+B)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('b'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyGitBranchHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('B'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyGitBranchHandler)),
    );

    // Directory Jumper (Ctrl+Z)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('z'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyZDirectoryHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('Z'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyZDirectoryHandler)),
    );

    // Project Switcher (Ctrl+O)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('o'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyProjectHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('O'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyProjectHandler)),
    );

    // Git Status Stager (Ctrl+G)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('g'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyGitStagerHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('G'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyGitStagerHandler)),
    );

    // Prompt Theme Config Wizard (Ctrl+T)
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('t'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyPromptThemeHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('T'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyPromptThemeHandler)),
    );

    rl.bind_sequence(
        KeyEvent(KeyCode::Char('a'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::Move(Movement::BeginningOfLine)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('e'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::Move(Movement::EndOfLine)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('l'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::ClearScreen),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('u'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyTrashVaultHandler)),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('U'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(FuzzyTrashVaultHandler)),
    );

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
