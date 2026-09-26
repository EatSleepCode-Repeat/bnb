use std::env;

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

pub fn run(args: &[String]) -> Result<(), String> {
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            let clean_val = value.trim_matches(|c| c == '\'' || c == '"');
            let expanded = expand_env(clean_val);
            env::set_var(key, expanded);
        }
    }
    Ok(())
}
