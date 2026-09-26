use std::env;
use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<(), String> {
    let target = if args.is_empty() {
        env::var("HOME").map_err(|_| "cd: HOME not set".to_string())?
    } else if args[0] == "-" {
        env::var("OLDPWD").map_err(|_| "cd: OLDPWD not set".to_string())?
    } else {
        args[0].clone()
    };

    let current_dir = env::current_dir().ok();

    let path = if target.starts_with('~') {
        if let Ok(home) = env::var("HOME") {
            PathBuf::from(target.replacen('~', &home, 1))
        } else {
            PathBuf::from(target)
        }
    } else {
        PathBuf::from(target)
    };

    env::set_current_dir(&path).map_err(|e| format!("cd: {}: {}", path.display(), e))?;

    if let Ok(new_pwd) = env::current_dir() {
        if let Some(old) = current_dir {
            if let Some(old_str) = old.to_str() {
                env::set_var("OLDPWD", old_str);
            }
        }
        if let Some(pwd_str) = new_pwd.to_str() {
            env::set_var("PWD", pwd_str);
        }
        crate::builtins::z::add_path(&new_pwd);
    }

    Ok(())
}
