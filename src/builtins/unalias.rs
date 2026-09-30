pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("unalias: not enough arguments".to_string());
    }

    if args.iter().any(|a| a == "-a") {
        crate::builtins::alias::remove_all_aliases();
        return Ok(());
    }

    let mut had_err = false;
    for arg in args {
        if !crate::builtins::alias::remove_alias(arg) {
            eprintln!("unalias: no such hash table element: {}", arg);
            had_err = true;
        }
    }

    if had_err {
        Err("unalias: one or more aliases not found".to_string())
    } else {
        Ok(())
    }
}
