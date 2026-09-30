use std::env;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("unset: not enough arguments".to_string());
    }

    for arg in args {
        env::remove_var(arg);
    }
    Ok(())
}
