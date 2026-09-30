use std::env;
use std::process::Command;
use std::time::Duration;
use terminal_size::{terminal_size, Width};

fn get_home_dir() -> Option<String> {
    env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
}

struct GitStatus {
    branch: String,
    dirty: bool,
    untracked: bool,
    ahead: u32,
    behind: u32,
}

pub fn get_prompt(last_duration: Option<Duration>, last_status: Option<i32>) -> String {
    let cwd = env::current_dir()
        .map(|p| {
            let path = p.display().to_string();
            if let Some(home) = get_home_dir() {
                if path.starts_with(&home) {
                    return path.replacen(&home, "~", 1);
                }
            }
            path
        })
        .unwrap_or_else(|_| "/".into());

    let git_status = get_git_status();

    let os_icon = if cfg!(target_os = "macos") {
        ""
    } else if cfg!(target_os = "linux") {
        ""
    } else {
        ""
    };

    let seg1_bg = "\x1b[48;5;238m";
    let seg1_fg = "\x1b[1;38;5;255m";
    let seg1_text = format!("{}{} {}  ", seg1_bg, seg1_fg, os_icon);

    let seg2_bg = "\x1b[48;5;75m";
    let seg2_fg = "\x1b[1;38;5;255m";
    let trans1 = format!("\x1b[38;5;238m{}\u{e0b0}", seg2_bg);
    let seg2_text = format!("{} 📁 {} ", seg2_fg, cwd);

    let (seg3_text, last_bg_fg) = match git_status {
        Some(gs) => {
            let is_dirty = gs.dirty || gs.untracked;
            let git_bg = if is_dirty {
                "\x1b[48;5;178m" // Amber/Gold if dirty
            } else {
                "\x1b[48;5;141m" // Purple if clean
            };
            let git_fg = "\x1b[1;38;5;232m";
            let trans2 = format!("\x1b[38;5;75m{}\u{e0b0}", git_bg);

            let mut flags = String::new();
            if gs.dirty {
                flags.push('*');
            }
            if gs.untracked {
                flags.push('?');
            }
            if gs.ahead > 0 {
                flags.push_str(&format!(" ⇡{}", gs.ahead));
            }
            if gs.behind > 0 {
                flags.push_str(&format!(" ⇣{}", gs.behind));
            }

            let flag_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" {}", flags)
            };

            let git_content = format!("{}{}  {}{} ", git_bg, git_fg, gs.branch, flag_str);
            (
                format!("{}{}", trans2, git_content),
                if is_dirty {
                    "\x1b[38;5;178m"
                } else {
                    "\x1b[38;5;141m"
                },
            )
        }
        None => (String::new(), "\x1b[38;5;75m"),
    };

    let left_bar_end = format!("\x1b[0m{}\u{e0b0}\x1b[0m", last_bg_fg);
    let left_bar = format!(
        "{}{}{}{}{}",
        seg1_text, trans1, seg2_text, seg3_text, left_bar_end
    );

    let status_badge = match last_status {
        Some(code) if code != 0 => {
            format!("\x1b[48;5;196m\x1b[1;38;5;255m ✘ {} \x1b[0m", code)
        }
        _ => String::new(),
    };

    let duration_badge = match last_duration {
        Some(d) if d.as_millis() >= 500 => {
            let duration_str = format_duration(d);
            format!(
                "\x1b[48;5;215m\x1b[1;38;5;232m took {} ⌛ \x1b[0m",
                duration_str
            )
        }
        _ => String::new(),
    };

    let right_badges = match (!status_badge.is_empty(), !duration_badge.is_empty()) {
        (true, true) => format!("{} {}", status_badge, duration_badge),
        (true, false) => status_badge,
        (false, true) => duration_badge,
        (false, false) => String::new(),
    };

    let term_width = terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80);

    let left_len = strip_ansi(&left_bar).chars().count() + 1;
    let right_len = if right_badges.is_empty() {
        0
    } else {
        strip_ansi(&right_badges).chars().count()
            + (if right_badges.contains('⌛') { 1 } else { 0 })
    };

    let line1 = if !right_badges.is_empty() && term_width > (left_len + right_len + 2) {
        let spaces_count = term_width.saturating_sub(left_len + right_len + 1);
        let spaces = " ".repeat(spaces_count);
        format!("{}{}{}", left_bar, spaces, right_badges)
    } else {
        format!("{}{}", left_bar, right_badges)
    };

    let prompt_arrow = if last_status.unwrap_or(0) != 0 {
        "\x1b[1;31m❯\x1b[0m "
    } else {
        "\x1b[1;32m❯\x1b[0m "
    };

    format!("{}\n{}", line1, prompt_arrow)
}

fn strip_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for c in input.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' || c == 'K' || c == 'H' || c == 'J' {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn get_git_status() -> Option<GitStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-b"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    let branch_line = lines.next()?;

    if !branch_line.starts_with("## ") {
        return None;
    }

    let branch_part = &branch_line[3..];
    let (branch_name, ahead, behind) = parse_branch_header(branch_part);

    let mut dirty = false;
    let mut untracked = false;

    for line in lines {
        if line.starts_with("??") {
            untracked = true;
        } else if !line.trim().is_empty() {
            dirty = true;
        }
    }

    Some(GitStatus {
        branch: branch_name,
        dirty,
        untracked,
        ahead,
        behind,
    })
}

fn parse_branch_header(header: &str) -> (String, u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;

    let branch_spec = if let Some((name_part, flags_part)) = header.split_once(" [") {
        let flags = flags_part.trim_end_matches(']');
        for part in flags.split(',') {
            let trimmed = part.trim();
            if let Some(num_str) = trimmed.strip_prefix("ahead ") {
                ahead = num_str.parse().unwrap_or(0);
            } else if let Some(num_str) = trimmed.strip_prefix("behind ") {
                behind = num_str.parse().unwrap_or(0);
            }
        }
        name_part
    } else {
        header
    };

    let branch_name = if let Some((local, _)) = branch_spec.split_once("...") {
        local.to_string()
    } else if branch_spec.starts_with("HEAD (no branch") {
        ":detached".to_string()
    } else {
        branch_spec.to_string()
    };

    (branch_name, ahead, behind)
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 60 {
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{}m {}s", mins, rem_secs)
    } else if secs >= 10 {
        format!("{}s", secs)
    } else if secs >= 1 {
        let tenths = d.subsec_millis() / 100;
        format!("{}.{}s", secs, tenths)
    } else {
        format!("{}ms", d.as_millis())
    }
}
