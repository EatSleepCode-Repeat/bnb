use std::io::Write;

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], writer: &mut dyn Write) -> Result<(), String> {
    let mut newline = true;
    let mut interpret_escapes = false;
    let mut idx = 0;

    while idx < args.len() {
        match args[idx].as_str() {
            "-n" => newline = false,
            "-e" => interpret_escapes = true,
            "-ne" | "-en" => {
                newline = false;
                interpret_escapes = true;
            }
            _ => break,
        }
        idx += 1;
    }

    let text_args = &args[idx..];
    let mut text = text_args.join(" ");

    if interpret_escapes {
        text = unescape(&text);
    }

    if newline {
        writeln!(writer, "{}", text).map_err(|e| e.to_string())?;
    } else {
        write!(writer, "{}", text).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn unescape(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}
