use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("source: filename argument required".to_string());
    }

    let expanded = crate::expander::expand_tilde(&args[0]);
    let path = PathBuf::from(&expanded);

    crate::config::parse_file(&path)
}
