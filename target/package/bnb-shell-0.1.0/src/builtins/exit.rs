use std::process;

pub fn run(args: &[String]) -> ! {
    let code = args.first().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    process::exit(code);
}