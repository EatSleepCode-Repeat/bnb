use std::io::Write;

#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<(), String> {
    run_with_writer(args, &mut std::io::stdout())
}

pub fn run_with_writer(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    let mut no_newline = false;
    let mut interpret_escapes = false;
    let mut start_idx = 0;

    while start_idx < args.len() {
        let arg = &args[start_idx];
        if arg.starts_with('-')
            && arg.len() > 1
            && arg[1..].chars().all(|c| c == 'n' || c == 'e' || c == 'E')
        {
            for c in arg[1..].chars() {
                match c {
                    'n' => no_newline = true,
                    'e' => interpret_escapes = true,
                    'E' => interpret_escapes = false,
                    _ => {}
                }
            }
            start_idx += 1;
        } else {
            break;
        }
    }

    let words = &args[start_idx..];
    let mut output = words.join(" ");

    if interpret_escapes {
        output = unescape_string(&output);
    }

    if no_newline {
        let _ = write!(out, "{}", output);
    } else {
        let _ = writeln!(out, "{}", output);
    }

    let _ = out.flush();
    Ok(())
}

fn unescape_string(input: &str) -> String {
    let mut res = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => res.push('\n'),
                Some('t') => res.push('\t'),
                Some('r') => res.push('\r'),
                Some('\\') => res.push('\\'),
                Some('e') => res.push('\x1b'),
                Some('a') => res.push('\x07'),
                Some('b') => res.push('\x08'),
                Some('0') => res.push('\0'),
                Some(other) => {
                    res.push('\\');
                    res.push(other);
                }
                None => res.push('\\'),
            }
        } else {
            res.push(c);
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape() {
        assert_eq!(unescape_string(r"hello\nworld"), "hello\nworld");
        assert_eq!(unescape_string(r"foo\tbar"), "foo\tbar");
    }
}

