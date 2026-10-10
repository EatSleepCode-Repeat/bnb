use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("rm: missing operand".to_string());
    }

    let mut force = false;
    let mut recursive = false;
    let mut targets = Vec::new();

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") {
            for c in arg.chars().skip(1) {
                match c {
                    'r' | 'R' => recursive = true,
                    'f' => force = true,
                    _ => {}
                }
            }
        } else if arg == "--recursive" {
            recursive = true;
        } else if arg == "--force" {
            force = true;
        } else {
            targets.push(arg);
        }
    }

    if targets.is_empty() {
        if force {
            return Ok(());
        }
        return Err("rm: missing operand".to_string());
    }

    for target_str in targets {
        let expanded = crate::expander::expand_tilde(target_str);
        let path = PathBuf::from(&expanded);

        if !path.exists() {
            if !force {
                eprintln!(
                    "rm: cannot remove '{}': No such file or directory",
                    target_str
                );
            }
            continue;
        }

        if path.is_dir() && !recursive {
            return Err(format!(
                "rm: cannot remove '{}': Is a directory",
                target_str
            ));
        }

        if let Err(e) = crate::safety::trash::move_to_trash(&path) {
            if !force {
                return Err(e);
            }
        }
    }

    Ok(())
}
