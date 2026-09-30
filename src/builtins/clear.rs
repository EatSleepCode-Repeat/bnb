use std::io::Write;

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(_args: &[String], out: &mut dyn Write) -> Result<(), String> {
    let _ = write!(out, "\x1b[2J\x1b[H");
    let _ = out.flush();
    Ok(())
}
