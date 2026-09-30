use std::env;
use std::path::PathBuf;

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
    }

    Ok(())
}
