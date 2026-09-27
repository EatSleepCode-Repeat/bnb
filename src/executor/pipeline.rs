use crate::expander::expand_args;
use crate::parser::ast::{Pipeline, Redirection};
use std::env;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn get_home_dir() -> Option<String> {
    env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
}

fn find_binary(cmd: &str) -> Option<PathBuf> {
    let path = Path::new(cmd);
    if path.is_absolute() || cmd.contains('/') || cmd.contains('\\') {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        #[cfg(target_os = "windows")]
        {
            let with_exe = path.with_extension("exe");
            if with_exe.is_file() {
                return Some(with_exe);
            }
        }
        return None;
    }

    let mut search_dirs = Vec::new();

    if let Ok(path_var) = env::var("PATH") {
        search_dirs.extend(env::split_paths(&path_var));
    }

    if cfg!(not(target_os = "windows")) {
        let default_dirs = [
            "/bin",
            "/usr/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/usr/sbin",
            "/sbin",
        ];
        for d in default_dirs {
            search_dirs.push(PathBuf::from(d));
        }
    }

    if let Some(home) = get_home_dir() {
        search_dirs.push(PathBuf::from(home).join(".cargo/bin"));
    }

    for dir in search_dirs {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(target_os = "windows")]
        {
            let cand_exe = dir.join(format!("{}.exe", cmd));
            if cand_exe.is_file() {
                return Some(cand_exe);
            }
        }
    }

    None
}

pub fn execute_pipeline(pipeline: &Pipeline) -> Result<i32, String> {
    let num_cmds = pipeline.commands.len();
    if num_cmds == 0 {
        return Ok(0);
    }

    if num_cmds == 1 {
        let cmd = &pipeline.commands[0];
        if cmd.args.is_empty() {
            return Ok(0);
        }

        let raw_args = &cmd.args[1..];
        let expanded_args = expand_args(raw_args);

        let binary_path = find_binary(&cmd.args[0]);

        let mut sys_cmd = match binary_path {
            Some(path) => {
                let mut c = Command::new(path);
                c.args(&expanded_args);
                c
            }
            None => {
                let full_cmd = format!("{} {}", cmd.args[0], expanded_args.join(" "));
                if cfg!(target_os = "windows") {
                    let mut c = Command::new("cmd.exe");
                    c.args(["/C", &full_cmd]);
                    c
                } else {
                    let mut c = Command::new("zsh");
                    c.args(["-c", &format!("source ~/.zshrc 2>/dev/null; {}", full_cmd)]);
                    c
                }
            }
        };

        for redir in &cmd.redirections {
            match redir {
                Redirection::OutputTruncate(path) => {
                    let file = File::create(path)
                        .map_err(|e| format!("bnb: cannot create file {}: {}", path, e))?;
                    sys_cmd.stdout(Stdio::from(file));
                }
                Redirection::OutputAppend(path) => {
                    let file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(|e| format!("bnb: cannot append to file {}: {}", path, e))?;
                    sys_cmd.stdout(Stdio::from(file));
                }
                Redirection::Input(path) => {
                    let file = File::open(path)
                        .map_err(|_| format!("bnb: {}: No such file or directory", path))?;
                    sys_cmd.stdin(Stdio::from(file));
                }
            }
        }

        if pipeline.run_in_background {
            let child = sys_cmd.spawn().map_err(|e| format!("bnb: error: {}", e))?;
            println!("[+] Job backgrounded PID: {}", child.id());
            return Ok(0);
        } else {
            let mut child = sys_cmd.spawn().map_err(|e| format!("bnb: error: {}", e))?;
            let status = child.wait().map_err(|e| format!("bnb: wait error: {}", e))?;
            return Ok(status.code().unwrap_or(1));
        }
    }

    let mut children = Vec::new();
    let mut prev_stdout: Option<Stdio> = None;

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        if cmd.args.is_empty() {
            continue;
        }

        let raw_args = &cmd.args[1..];
        let expanded_args = expand_args(raw_args);

        let binary_path = find_binary(&cmd.args[0]);
        let mut sys_cmd = match binary_path {
            Some(path) => {
                let mut c = Command::new(path);
                c.args(&expanded_args);
                c
            }
            None => {
                let full_cmd = format!("{} {}", cmd.args[0], expanded_args.join(" "));
                if cfg!(target_os = "windows") {
                    let mut c = Command::new("cmd.exe");
                    c.args(["/C", &full_cmd]);
                    c
                } else {
                    let mut c = Command::new("zsh");
                    c.args(["-c", &format!("source ~/.zshrc 2>/dev/null; {}", full_cmd)]);
                    c
                }
            }
        };

        if let Some(stdin) = prev_stdout.take() {
            sys_cmd.stdin(stdin);
        }

        if i < num_cmds - 1 {
            sys_cmd.stdout(Stdio::piped());
        }

        for redir in &cmd.redirections {
            match redir {
                Redirection::OutputTruncate(path) => {
                    if let Ok(file) = File::create(path) {
                        sys_cmd.stdout(Stdio::from(file));
                    }
                }
                Redirection::OutputAppend(path) => {
                    if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
                        sys_cmd.stdout(Stdio::from(file));
                    }
                }
                Redirection::Input(path) => {
                    if let Ok(file) = File::open(path) {
                        sys_cmd.stdin(Stdio::from(file));
                    }
                }
            }
        }

        let mut child = sys_cmd.spawn().map_err(|e| format!("bnb: error: {}", e))?;

        if i < num_cmds - 1 {
            if let Some(stdout) = child.stdout.take() {
                prev_stdout = Some(Stdio::from(stdout));
            }
        }

        children.push(child);
    }

    let mut last_code = 0;
    if !pipeline.run_in_background {
        for mut child in children {
            if let Ok(status) = child.wait() {
                last_code = status.code().unwrap_or(0);
            }
        }
    } else if let Some(last_child) = children.last() {
        println!("[+] Job backgrounded PID: {}", last_child.id());
    }

    Ok(last_code)
}
