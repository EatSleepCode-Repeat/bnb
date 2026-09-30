use std::env;
use std::io::Write;

pub fn expand_env(val: &str) -> String {
    let mut result = val.to_string();
    if result.contains('$') {
        let vars: Vec<(String, String)> = env::vars().collect();
        for (k, v) in vars {
            let var_str = format!("${}", k);
            let var_brace = format!("${{{}}}", k);
            result = result.replace(&var_str, &v);
            result = result.replace(&var_brace, &v);
        }
    }
    result
}

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    if args.is_empty() {
        let mut vars: Vec<(String, String)> = env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in vars {
            let _ = writeln!(out, "export {}=\"{}\"", k, v);
        }
        return Ok(());
    }

    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            let clean_key = key.trim();
            let clean_val = value.trim().trim_matches(|c| c == '\'' || c == '"');
            let expanded = expand_env(clean_val);
            env::set_var(clean_key, expanded);
        } else {
            if env::var(arg).is_err() {
                env::set_var(arg, "");
            }
        }
    }
    Ok(())
}
