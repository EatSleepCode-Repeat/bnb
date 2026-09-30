use crate::expander::expand_args;
use crate::parser::ast::{ChainOp, Command, CommandChain, Pipeline, Redirection};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as SysCommand, Stdio};

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
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
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

pub fn resolve_command_alias(cmd: &mut Command) {
    if cmd.args.is_empty() {
        return;
    }

    if let Some(aliased) = crate::builtins::alias::resolve(&cmd.args[0]) {
        let tokens = crate::parser::lexer::tokenize(&aliased, 0);
        let mut words: Vec<String> = tokens
            .into_iter()
            .filter_map(|t| match t {
                crate::parser::lexer::Token::Word(w) => Some(w),
                _ => None,
            })
            .collect();

        if !words.is_empty() {
            let extra = cmd.args[1..].to_vec();
            words.extend(extra);
            cmd.args = words;
        }
    }
}

#[allow(dead_code)]
pub fn execute_chain(chain: &CommandChain) -> Result<i32, String> {
    let mut last_code = 0;
    let mut should_run = true;

    for item in &chain.chains {
        if should_run {
            match execute_pipeline(&item.pipeline) {
                Ok(code) => {
                    last_code = code;
                }
                Err(e) => {
                    eprintln!("{}", e);
                    last_code = 1;
                }
            }
        }

        match item.next_op {
            Some(ChainOp::And) => {
                should_run = last_code == 0;
            }
            Some(ChainOp::Or) => {
                should_run = last_code != 0;
            }
            Some(ChainOp::Sequence) => {
                should_run = true;
            }
            None => break,
        }
    }

    Ok(last_code)
}

fn execute_builtin_single(cmd: &Command) -> Result<i32, String> {
    let mut file_out: Option<Box<dyn Write>> = None;
    let mut file_err: Option<Box<dyn Write>> = None;

    for redir in &cmd.redirections {
        match redir {
            Redirection::OutputTruncate(path) => {
                let f = File::create(path)
                    .map_err(|e| format!("bnb: cannot create file {}: {}", path, e))?;
                file_out = Some(Box::new(f));
            }
            Redirection::OutputAppend(path) => {
                let f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| format!("bnb: cannot append to file {}: {}", path, e))?;
                file_out = Some(Box::new(f));
            }
            Redirection::StderrTruncate(path) => {
                let f = File::create(path)
                    .map_err(|e| format!("bnb: cannot create file {}: {}", path, e))?;
                file_err = Some(Box::new(f));
            }
            Redirection::StderrAppend(path) => {
                let f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| format!("bnb: cannot append to file {}: {}", path, e))?;
                file_err = Some(Box::new(f));
            }
            Redirection::OutputAndStderr(path) => {
                let f = File::create(path)
                    .map_err(|e| format!("bnb: cannot create file {}: {}", path, e))?;
                let f_clone = f.try_clone().map_err(|e| e.to_string())?;
                file_out = Some(Box::new(f));
                file_err = Some(Box::new(f_clone));
            }
            Redirection::Input(_) => {}
        }
    }

    let mut stdout_handle = io::stdout();
    let mut stderr_handle = io::stderr();

    let out_writer: &mut dyn Write = match file_out.as_mut() {
        Some(w) => w.as_mut(),
        None => &mut stdout_handle,
    };

    let err_writer: &mut dyn Write = match file_err.as_mut() {
        Some(w) => w.as_mut(),
        None => &mut stderr_handle,
    };

    let expanded_args = expand_args(&cmd.args[1..]);
    match crate::builtins::execute_with_writer(&cmd.args[0], &expanded_args, out_writer) {
        Ok(()) => Ok(0),
        Err(e) => {
            let _ = writeln!(err_writer, "{}", e);
            Ok(1)
        }
    }
}

pub fn execute_pipeline(pipeline: &Pipeline) -> Result<i32, String> {
    let num_cmds = pipeline.commands.len();
    if num_cmds == 0 {
        return Ok(0);
    }

    if num_cmds == 1 {
        let mut cmd = pipeline.commands[0].clone();
        if cmd.args.is_empty() {
            return Ok(0);
        }

        resolve_command_alias(&mut cmd);

        if crate::builtins::is_builtin(&cmd.args[0]) {
            return execute_builtin_single(&cmd);
        }

        let raw_args = &cmd.args[1..];
        let expanded_args = expand_args(raw_args);

        let binary_path = find_binary(&cmd.args[0]);

        let mut sys_cmd = match binary_path {
            Some(path) => {
                let mut c = SysCommand::new(path);
                c.args(&expanded_args);
                c
            }
            None => {
                let full_cmd = format!("{} {}", cmd.args[0], expanded_args.join(" "));
                if cfg!(target_os = "windows") {
                    let mut c = SysCommand::new("cmd.exe");
                    c.args(["/C", &full_cmd]);
                    c
                } else {
                    let mut c = SysCommand::new("zsh");
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
                Redirection::StderrTruncate(path) => {
                    let file = File::create(path)
                        .map_err(|e| format!("bnb: cannot create file {}: {}", path, e))?;
                    sys_cmd.stderr(Stdio::from(file));
                }
                Redirection::StderrAppend(path) => {
                    let file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(|e| format!("bnb: cannot append to file {}: {}", path, e))?;
                    sys_cmd.stderr(Stdio::from(file));
                }
                Redirection::OutputAndStderr(path) => {
                    let file = File::create(path)
                        .map_err(|e| format!("bnb: cannot create file {}: {}", path, e))?;
                    let err_file = file.try_clone().map_err(|e| e.to_string())?;
                    sys_cmd.stdout(Stdio::from(file));
                    sys_cmd.stderr(Stdio::from(err_file));
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

    // Multi-command pipeline
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<Stdio> = None;
    let mut builtin_buffer: Option<Vec<u8>> = None;

    for (i, cmd_ref) in pipeline.commands.iter().enumerate() {
        let mut cmd = cmd_ref.clone();
        if cmd.args.is_empty() {
            continue;
        }

        resolve_command_alias(&mut cmd);

        let raw_args = &cmd.args[1..];
        let expanded_args = expand_args(raw_args);

        // If first command in pipeline is a builtin:
        if i == 0 && crate::builtins::is_builtin(&cmd.args[0]) {
            let mut buf = Vec::new();
            let _ = crate::builtins::execute_with_writer(&cmd.args[0], &expanded_args, &mut buf);
            builtin_buffer = Some(buf);
            continue;
        }

        let binary_path = find_binary(&cmd.args[0]);
        let mut sys_cmd = match binary_path {
            Some(path) => {
                let mut c = SysCommand::new(path);
                c.args(&expanded_args);
                c
            }
            None => {
                let full_cmd = format!("{} {}", cmd.args[0], expanded_args.join(" "));
                if cfg!(target_os = "windows") {
                    let mut c = SysCommand::new("cmd.exe");
                    c.args(["/C", &full_cmd]);
                    c
                } else {
                    let mut c = SysCommand::new("zsh");
                    c.args(["-c", &format!("source ~/.zshrc 2>/dev/null; {}", full_cmd)]);
                    c
                }
            }
        };

        if let Some(buf) = builtin_buffer.take() {
            sys_cmd.stdin(Stdio::piped());
            let mut child = sys_cmd.spawn().map_err(|e| format!("bnb: error: {}", e))?;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(&buf);
            }
            children.push(child);
            continue;
        } else if let Some(stdin) = prev_stdout.take() {
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
                Redirection::StderrTruncate(path) => {
                    if let Ok(file) = File::create(path) {
                        sys_cmd.stderr(Stdio::from(file));
                    }
                }
                Redirection::StderrAppend(path) => {
                    if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
                        sys_cmd.stderr(Stdio::from(file));
                    }
                }
                Redirection::OutputAndStderr(path) => {
                    if let Ok(file) = File::create(path) {
                        if let Ok(err_file) = file.try_clone() {
                            sys_cmd.stdout(Stdio::from(file));
                            sys_cmd.stderr(Stdio::from(err_file));
                        }
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
