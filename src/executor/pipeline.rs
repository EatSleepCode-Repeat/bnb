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
        let Ok(tokens) = crate::parser::lexer::tokenize(&aliased, 0) else {
            return;
        };
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

fn execute_builtin_single(cmd: &Command, assignments: &[(String, String)]) -> Result<i32, String> {
    let previous_values = apply_temporary_assignments(assignments);
    let result = execute_builtin_single_inner(cmd);
    restore_temporary_assignments(previous_values);
    result
}

fn execute_builtin_single_inner(cmd: &Command) -> Result<i32, String> {
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
    let mut pipeline = pipeline.clone();
    if pipeline.commands.is_empty() {
        return Ok(0);
    }

    let mut assignments = Vec::with_capacity(pipeline.commands.len());
    for command in &mut pipeline.commands {
        let mut command_assignments = Vec::new();
        while let Some(word) = command.args.first() {
            let Some((name, value)) = split_assignment(word) else {
                break;
            };
            command_assignments.push((
                name.to_string(),
                crate::expander::restore_quoted_chars(value),
            ));
            command.args.remove(0);
        }
        assignments.push(command_assignments);
    }

    if pipeline.commands.len() == 1 && pipeline.commands[0].args.is_empty() {
        for (name, value) in assignments.into_iter().next().unwrap_or_default() {
            env::set_var(name, value);
        }
        return Ok(0);
    }

    execute_pipeline_inner(&pipeline, &assignments)
}

fn split_assignment(word: &str) -> Option<(&str, &str)> {
    let (name, value) = word.split_once('=')?;
    let mut chars = name.chars();
    if !matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        || !chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((name, value))
}

fn apply_temporary_assignments(assignments: &[(String, String)]) -> Vec<(String, Option<String>)> {
    let previous = assignments
        .iter()
        .map(|(name, _)| (name.clone(), env::var(name).ok()))
        .collect();
    for (name, value) in assignments {
        env::set_var(name, value);
    }
    previous
}

fn restore_temporary_assignments(previous: Vec<(String, Option<String>)>) {
    for (name, value) in previous {
        if let Some(value) = value {
            env::set_var(name, value);
        } else {
            env::remove_var(name);
        }
    }
}

fn execute_pipeline_inner(
    pipeline: &Pipeline,
    assignments: &[Vec<(String, String)>],
) -> Result<i32, String> {
    let num_cmds = pipeline.commands.len();
    if num_cmds == 0 {
        return Ok(0);
    }

    for command in &pipeline.commands {
        let mut resolved = command.clone();
        resolve_command_alias(&mut resolved);
        if let Some(program) = resolved.args.first() {
            if !crate::builtins::is_builtin(program) && find_binary(program).is_none() {
                return Err(format!("bnb: command not found: {}", program));
            }
        }
    }

    if num_cmds == 1 {
        let mut cmd = pipeline.commands[0].clone();
        if cmd.args.is_empty() {
            return Ok(0);
        }

        resolve_command_alias(&mut cmd);

        if crate::builtins::is_builtin(&cmd.args[0]) {
            return execute_builtin_single(&cmd, &assignments[0]);
        }

        let raw_args = &cmd.args[1..];
        let expanded_args = expand_args(raw_args);

        let binary_path = find_binary(&cmd.args[0]);

        let binary_path =
            binary_path.ok_or_else(|| format!("bnb: command not found: {}", cmd.args[0]))?;
        let mut sys_cmd = SysCommand::new(binary_path);
        sys_cmd.args(&expanded_args);
        sys_cmd.envs(assignments[0].iter().cloned());

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
            let status = child
                .wait()
                .map_err(|e| format!("bnb: wait error: {}", e))?;
            return Ok(status.code().unwrap_or(1));
        }
    }

    // Multi-command pipeline
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<Stdio> = None;
    let mut builtin_buffer: Option<Vec<u8>> = None;
    let mut builtin_final_status = None;

    for (i, cmd_ref) in pipeline.commands.iter().enumerate() {
        let mut cmd = cmd_ref.clone();
        if cmd.args.is_empty() {
            continue;
        }

        resolve_command_alias(&mut cmd);

        let raw_args = &cmd.args[1..];
        let expanded_args = expand_args(raw_args);

        if crate::builtins::is_builtin(&cmd.args[0]) {
            drop(prev_stdout.take());
            if i == num_cmds - 1 {
                builtin_final_status = Some(execute_builtin_single(&cmd, &assignments[i])?);
            } else {
                let mut output = Vec::new();
                let previous_values = apply_temporary_assignments(&assignments[i]);
                let result =
                    crate::builtins::execute_with_writer(&cmd.args[0], &expanded_args, &mut output);
                restore_temporary_assignments(previous_values);
                result.map_err(|error| format!("bnb: {}", error))?;
                builtin_buffer = Some(output);
            }
            continue;
        }

        let binary_path = find_binary(&cmd.args[0]);
        let binary_path =
            binary_path.ok_or_else(|| format!("bnb: command not found: {}", cmd.args[0]))?;
        let mut sys_cmd = SysCommand::new(binary_path);
        sys_cmd.args(&expanded_args);
        sys_cmd.envs(assignments[i].iter().cloned());

        let pending_builtin_input = if let Some(buf) = builtin_buffer.take() {
            sys_cmd.stdin(Stdio::piped());
            Some(buf)
        } else if let Some(stdin) = prev_stdout.take() {
            sys_cmd.stdin(stdin);
            None
        } else {
            None
        };

        if i < num_cmds - 1 {
            sys_cmd.stdout(Stdio::piped());
        }

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
                    let err_file = file
                        .try_clone()
                        .map_err(|e| format!("bnb: cannot duplicate file {}: {}", path, e))?;
                    sys_cmd.stdout(Stdio::from(file));
                    sys_cmd.stderr(Stdio::from(err_file));
                }
                Redirection::Input(path) => {
                    let file = File::open(path)
                        .map_err(|e| format!("bnb: cannot open file {}: {}", path, e))?;
                    sys_cmd.stdin(Stdio::from(file));
                }
            }
        }

        let mut child = sys_cmd.spawn().map_err(|e| format!("bnb: error: {}", e))?;
        if let Some(buf) = pending_builtin_input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(&buf)
                    .map_err(|error| format!("bnb: pipeline write error: {}", error))?;
            }
        }

        if i < num_cmds - 1 {
            if let Some(stdout) = child.stdout.take() {
                prev_stdout = Some(Stdio::from(stdout));
            }
        }

        children.push(child);
    }

    let mut last_code = builtin_final_status.unwrap_or(0);
    if !pipeline.run_in_background {
        for mut child in children {
            if let Ok(status) = child.wait() {
                last_code = status.code().unwrap_or(0);
            }
        }
        if let Some(status) = builtin_final_status {
            last_code = status;
        }
    } else if let Some(last_child) = children.last() {
        println!("[+] Job backgrounded PID: {}", last_child.id());
    }

    Ok(last_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_only_command_persists_in_shell_environment() {
        let name = format!("BNB_ASSIGNMENT_TEST_{}", std::process::id());
        let pipeline = crate::parser::parse_pipeline(&format!("{}=value", name), 0).unwrap();
        execute_pipeline(&pipeline).unwrap();
        assert_eq!(env::var(&name).unwrap(), "value");
        env::remove_var(name);
    }

    #[test]
    fn command_prefix_assignment_is_restored_after_execution() {
        let name = format!("BNB_PREFIX_TEST_{}", std::process::id());
        env::remove_var(&name);
        let pipeline =
            crate::parser::parse_pipeline(&format!("{}=temporary echo test", name), 0).unwrap();
        execute_pipeline(&pipeline).unwrap();
        assert!(env::var(&name).is_err());
    }

    #[test]
    fn pipeline_stage_assignments_are_scoped_to_that_command() {
        let name = format!("BNB_PIPELINE_ASSIGNMENT_{}", std::process::id());
        let path = env::temp_dir().join(format!("bnb-pipeline-env-{}.txt", std::process::id()));
        let command = format!("echo input | {}=stage env > {}", name, path.display());
        let pipeline = crate::parser::parse_pipeline(&command, 0).unwrap();
        assert_eq!(execute_pipeline(&pipeline).unwrap(), 0);
        let output = std::fs::read_to_string(&path).unwrap();
        assert!(output.lines().any(|line| line == format!("{}=stage", name)));
        assert!(env::var(&name).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn builtins_can_appear_at_the_end_of_pipelines() {
        let path = env::temp_dir().join(format!("bnb-pipeline-builtin-{}.txt", std::process::id()));
        let command = format!("printf input | echo final > {}", path.display());
        let pipeline = crate::parser::parse_pipeline(&command, 0).unwrap();
        assert_eq!(execute_pipeline(&pipeline).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "final\n");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn builtin_output_can_feed_a_later_external_command() {
        let path = env::temp_dir().join(format!("bnb-pipeline-middle-{}.txt", std::process::id()));
        let command = format!("echo ignored | echo middle | cat > {}", path.display());
        let pipeline = crate::parser::parse_pipeline(&command, 0).unwrap();
        assert_eq!(execute_pipeline(&pipeline).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "middle\n");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_pipeline_command_does_not_start_earlier_stages() {
        let path = env::temp_dir().join(format!("bnb-preflight-{}.txt", std::process::id()));
        let command = format!("touch {} | bnb-command-missing-7742", path.display());
        let pipeline = crate::parser::parse_pipeline(&command, 0).unwrap();
        assert!(execute_pipeline(&pipeline).is_err());
        assert!(!path.exists());
    }
}
