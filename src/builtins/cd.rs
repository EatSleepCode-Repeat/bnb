use std::env;
use std::fs;
use std::path::PathBuf;

pub fn check_and_load_dotenv() {
    if let Ok(pwd) = env::current_dir() {
        let dotenv_path = pwd.join(".env");
        if dotenv_path.is_file() {
            if let Ok(content) = fs::read_to_string(&dotenv_path) {
                let mut count = 0;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    let kv = trimmed.strip_prefix("export ").unwrap_or(trimmed);
                    if let Some((k, v)) = kv.split_once('=') {
                        let clean_k = k.trim();
                        let clean_v = v.trim().trim_matches(|c| c == '\'' || c == '"');
                        if !clean_k.is_empty() {
                            env::set_var(clean_k, clean_v);
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    println!("\x1b[1;32m⚡ Auto-loaded {} variable(s) from .env\x1b[0m", count);
                }
            }
        }
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let is_dash = !args.is_empty() && args[0] == "-";
    let target = if args.is_empty() {
        env::var("HOME").map_err(|_| "cd: HOME not set".to_string())?
    } else if is_dash {
        env::var("OLDPWD").map_err(|_| "cd: OLDPWD not set".to_string())?
    } else {
        crate::expander::expand_tilde(&args[0])
    };

    let current_dir = env::current_dir().ok();
    let path = PathBuf::from(&target);

    env::set_current_dir(&path).map_err(|e| format!("cd: {}: {}", path.display(), e))?;

    if let Ok(new_pwd) = env::current_dir() {
        if let Some(old) = current_dir {
            if let Some(old_str) = old.to_str() {
                env::set_var("OLDPWD", old_str);
            }
        }
        if let Some(pwd_str) = new_pwd.to_str() {
            env::set_var("PWD", pwd_str);
            if is_dash {
                println!("{}", pwd_str);
            }
        }
        crate::builtins::z::add_path(&new_pwd);
        check_and_load_dotenv();
    }

    Ok(())
}