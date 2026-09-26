pub mod alias;
pub mod cd;
pub mod exit;
pub mod export;
pub mod z;

pub fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "cd" | "export" | "alias" | "exit" | "z")
}

pub fn execute(cmd: &str, args: &[String]) -> Result<(), String> {
    match cmd {
        "cd" => cd::run(args),
        "export" => export::run(args),
        "alias" => alias::run(args),
        "exit" => exit::run(args),
        "z" => z::run(args),
        _ => Err(format!("bnb: unknown builtin: {}", cmd)),
    }
}
