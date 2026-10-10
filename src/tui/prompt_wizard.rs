use crate::prompt::{load_prompt_config, save_prompt_config, ColorScheme};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use std::io::{self, stdout, Write};

struct RawGuard;

impl RawGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn run_wizard() -> Result<(), String> {
    let mut config = load_prompt_config();
    let _guard = RawGuard::enter().map_err(|e| format!("prompt-wizard: raw mode error: {}", e))?;
    let mut stdout = stdout();

    let styles = ["powerline", "lean", "minimal"];
    let separators = ["angled", "rounded", "slanted", "none"];
    let schemes = [
        ColorScheme::Classic,
        ColorScheme::Catppuccin,
        ColorScheme::Dracula,
        ColorScheme::Nord,
        ColorScheme::Gruvbox,
        ColorScheme::TokyoNight,
    ];

    let mut step: usize = 0; // 0: Style, 1: Separator, 2: Icons, 3: Scheme

    let mut style_idx = styles.iter().position(|&s| s == config.style).unwrap_or(0);
    let mut sep_idx = separators
        .iter()
        .position(|&s| s == config.separator)
        .unwrap_or(0);
    let mut scheme_idx = schemes
        .iter()
        .position(|&s| s == config.color_scheme)
        .unwrap_or(0);

    loop {
        config.style = styles[style_idx].to_string();
        config.separator = separators[sep_idx].to_string();
        config.color_scheme = schemes[scheme_idx];

        let (cols, _) = size().unwrap_or((80, 24));
        let width = (cols as usize).clamp(50, 78);

        write!(
            stdout,
            "\x1b[2J\x1b[1;1H\x1b[1;36m╭{}╮\r\n",
            "─".repeat(width - 2)
        )
        .map_err(|e| e.to_string())?;

        write!(
            stdout,
            "\x1b[1;36m│\x1b[1;37m  ⚙️  Prompt Configuration Wizard (Step {}/4)  \x1b[1;36m{}│\r\n",
            step + 1,
            " ".repeat(width.saturating_sub(47))
        )
        .map_err(|e| e.to_string())?;

        write!(stdout, "\x1b[1;36m├{}┤\r\n", "─".repeat(width - 2)).map_err(|e| e.to_string())?;

        match step {
            0 => {
                write!(
                    stdout,
                    "\x1b[1;36m│ \x1b[1;33mSelect Prompt Style:\x1b[0m{}\x1b[1;36m│\r\n",
                    " ".repeat(width.saturating_sub(22))
                )
                .map_err(|e| e.to_string())?;

                for (i, &st) in styles.iter().enumerate() {
                    let mark = if i == style_idx {
                        "❯ \x1b[1;32m"
                    } else {
                        "  \x1b[37m"
                    };
                    let line = format!("{}{:<12}\x1b[0m", mark, st);
                    write!(
                        stdout,
                        "\x1b[1;36m│\x1b[0m   {} {}\x1b[1;36m│\r\n",
                        line,
                        " ".repeat(width.saturating_sub(20))
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            1 => {
                write!(
                    stdout,
                    "\x1b[1;36m│ \x1b[1;33mSelect Powerline Separator:\x1b[0m{}\x1b[1;36m│\r\n",
                    " ".repeat(width.saturating_sub(29))
                )
                .map_err(|e| e.to_string())?;

                for (i, &sp) in separators.iter().enumerate() {
                    let mark = if i == sep_idx {
                        "❯ \x1b[1;32m"
                    } else {
                        "  \x1b[37m"
                    };
                    let line = format!("{}{:<12}\x1b[0m", mark, sp);
                    write!(
                        stdout,
                        "\x1b[1;36m│\x1b[0m   {} {}\x1b[1;36m│\r\n",
                        line,
                        " ".repeat(width.saturating_sub(20))
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            2 => {
                write!(
                    stdout,
                    "\x1b[1;36m│ \x1b[1;33mEnable Nerd Font Glyphs & Icons:\x1b[0m{}\x1b[1;36m│\r\n",
                    " ".repeat(width.saturating_sub(34))
                )
                .map_err(|e| e.to_string())?;

                let status = if config.enable_icons {
                    "\x1b[1;32m[✓] Enabled (Nerd Font)\x1b[0m"
                } else {
                    "\x1b[1;31m[ ] Disabled (Standard ASCII)\x1b[0m"
                };

                write!(
                    stdout,
                    "\x1b[1;36m│\x1b[0m   ❯ {} {}\x1b[1;36m│\r\n",
                    status,
                    " ".repeat(width.saturating_sub(35))
                )
                .map_err(|e| e.to_string())?;
            }
            3 => {
                write!(
                    stdout,
                    "\x1b[1;36m│ \x1b[1;33mSelect Color Scheme Preset:\x1b[0m{}\x1b[1;36m│\r\n",
                    " ".repeat(width.saturating_sub(29))
                )
                .map_err(|e| e.to_string())?;

                for (i, sch) in schemes.iter().enumerate() {
                    let mark = if i == scheme_idx {
                        "❯ \x1b[1;32m"
                    } else {
                        "  \x1b[37m"
                    };
                    let line = format!("{}{:<18}\x1b[0m", mark, sch.name());
                    write!(
                        stdout,
                        "\x1b[1;36m│\x1b[0m   {} {}\x1b[1;36m│\r\n",
                        line,
                        " ".repeat(width.saturating_sub(26))
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            _ => {}
        }

        write!(stdout, "\x1b[1;36m├{}┤\r\n", "─".repeat(width - 2)).map_err(|e| e.to_string())?;

        write!(
            stdout,
            "\x1b[1;36m│ \x1b[1;37mLive Preview:\x1b[0m{}\x1b[1;36m│\r\n",
            " ".repeat(width.saturating_sub(16))
        )
        .map_err(|e| e.to_string())?;

        let sample_prompt = crate::prompt::render_prompt_with_config(&config, None, Some(0));
        for line in sample_prompt.lines() {
            write!(stdout, "\x1b[1;36m│\x1b[0m  {} \r\n", line).map_err(|e| e.to_string())?;
        }

        write!(stdout, "\x1b[1;36m├{}┤\r\n", "─".repeat(width - 2)).map_err(|e| e.to_string())?;

        let nav_text = " [↑/↓] Change Option   [Enter] Next Step   [Esc] Save & Exit ";
        write!(
            stdout,
            "\x1b[1;36m╰─\x1b[1;37m{}\x1b[0m\x1b[1;36m{}╯\r\n",
            nav_text,
            "─".repeat(width.saturating_sub(nav_text.len() + 2))
        )
        .map_err(|e| e.to_string())?;

        stdout.flush().map_err(|e| e.to_string())?;

        if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    let _ = save_prompt_config(&config);
                    break;
                }
                KeyCode::Up => match step {
                    0 => style_idx = style_idx.saturating_sub(1),
                    1 => sep_idx = sep_idx.saturating_sub(1),
                    2 => config.enable_icons = !config.enable_icons,
                    3 => scheme_idx = scheme_idx.saturating_sub(1),
                    _ => {}
                },
                KeyCode::Down => match step {
                    0 if style_idx + 1 < styles.len() => style_idx += 1,
                    1 if sep_idx + 1 < separators.len() => sep_idx += 1,
                    2 => config.enable_icons = !config.enable_icons,
                    3 if scheme_idx + 1 < schemes.len() => scheme_idx += 1,
                    _ => {}
                },
                KeyCode::Enter | KeyCode::Right => {
                    if step < 3 {
                        step += 1;
                    } else {
                        let _ = save_prompt_config(&config);
                        break;
                    }
                }
                KeyCode::Left => {
                    step = step.saturating_sub(1);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
