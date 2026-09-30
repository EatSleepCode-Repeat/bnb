use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};

fn find_executable(cmd: &str) -> Option<PathBuf> {
    let p = Path::new(cmd);
    if p.is_file() {
        return Some(p.to_path_buf());
    }

    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
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
    }

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
        let candidate = PathBuf::from(d).join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    if args.is_empty() {
        return Err("which: not enough arguments".to_string());
    }

    let mut all_found = true;

    for arg in args {
        if let Some(aliased) = crate::builtins::alias::resolve(arg) {
            let _ = writeln!(out, "{} is an alias for {}", arg, aliased);
        } else if crate::builtins::is_builtin(arg) {
            let _ = writeln!(out, "{} is a shell builtin", arg);
        } else if let Some(path) = find_executable(arg) {
            let _ = writeln!(out, "{}", path.display());
        } else {
            eprintln!("{}: not found", arg);
            all_found = false;
        }
    }

    if all_found {
        Ok(())
    } else {
        Err("which: command not found".to_string())
    }
}
