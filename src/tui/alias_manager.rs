use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use std::fs;
use std::io::{self, stdout, Write};
use std::path::PathBuf;

use crate::tui::fuzzy::visual_width;

#[derive(Clone, Debug)]
pub struct AliasEntry {
    pub name: String,
    pub command: String,
}

enum Mode {
    List,
    Form {
        is_editing: bool,
        name_input: String,
        cmd_input: String,
        active_field: usize, // 0 = Name, 1 = Command
        error_msg: Option<String>,
    },
    ConfirmDelete,
}

struct TerminalGuard {
    alternate_screen: bool,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self {
            alternate_screen: false,
        })
    }

    fn enter_alternate_screen(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        stdout.execute(EnterAlternateScreen)?;
        self.alternate_screen = true;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.alternate_screen {
            let _ = stdout().execute(LeaveAlternateScreen);
        }
        let _ = disable_raw_mode();
    }
}

pub struct AliasManager;

impl AliasManager {
    pub fn run() -> Result<bool, String> {
        let bnbrc_path = get_bnbrc_path()?;
        let mut aliases = load_aliases_from_file(&bnbrc_path)?;

        let mut terminal =
            TerminalGuard::enter().map_err(|e| format!("alias manager: raw mode failed: {}", e))?;
        let mut stdout = stdout();
        terminal
            .enter_alternate_screen(&mut stdout)
            .map_err(|e| format!("alias manager: alternate screen failed: {}", e))?;

        let mut selected = 0;
        let mut mode = Mode::List;
        let mut modified = false;

        loop {
            let (term_cols, _) = size().unwrap_or((80, 24));
            let box_width = (term_cols as usize).saturating_sub(4).clamp(52, 80);
            let content_width = box_width.saturating_sub(4);

            if selected >= aliases.len() && !aliases.is_empty() {
                selected = aliases.len() - 1;
            }

            match &mode {
                Mode::List => {
                    render_list_view(&mut stdout, &aliases, selected, box_width, content_width)?;
                }
                Mode::Form {
                    is_editing,
                    name_input,
                    cmd_input,
                    active_field,
                    error_msg,
                } => {
                    render_form_view(
                        &mut stdout,
                        *is_editing,
                        name_input,
                        cmd_input,
                        *active_field,
                        error_msg.as_deref(),
                        box_width,
                        content_width,
                    )?;
                }
                Mode::ConfirmDelete => {
                    render_delete_confirm(&mut stdout, &aliases[selected], box_width)?;
                }
            }

            stdout.flush().map_err(|e| e.to_string())?;

            let event = event::read().map_err(|e| e.to_string())?;
            if let Event::Key(key) = event {
                match &mut mode {
                    Mode::List => match (key.code, key.modifiers) {
                        (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                            selected = selected.saturating_sub(1);
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                            if selected + 1 < aliases.len() {
                                selected += 1;
                            }
                        }
                        (KeyCode::Char('a'), KeyModifiers::NONE)
                        | (KeyCode::Char('n'), KeyModifiers::NONE) => {
                            mode = Mode::Form {
                                is_editing: false,
                                name_input: String::new(),
                                cmd_input: String::new(),
                                active_field: 0,
                                error_msg: None,
                            };
                        }
                        (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, _) => {
                            if let Some(entry) = aliases.get(selected) {
                                mode = Mode::Form {
                                    is_editing: true,
                                    name_input: entry.name.clone(),
                                    cmd_input: entry.command.clone(),
                                    active_field: 1,
                                    error_msg: None,
                                };
                            }
                        }
                        (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _)
                            if !aliases.is_empty() =>
                        {
                            mode = Mode::ConfirmDelete;
                        }
                        _ => {}
                    },
                    Mode::ConfirmDelete => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            aliases.remove(selected);
                            modified = true;
                            if selected >= aliases.len() && !aliases.is_empty() {
                                selected = aliases.len() - 1;
                            }
                            mode = Mode::List;
                        }
                        _ => mode = Mode::List,
                    },
                    Mode::Form {
                        is_editing,
                        name_input,
                        cmd_input,
                        active_field,
                        error_msg,
                    } => match key.code {
                        KeyCode::Esc => mode = Mode::List,
                        KeyCode::Tab => {
                            *active_field = if *active_field == 0 { 1 } else { 0 };
                        }
                        KeyCode::Enter => {
                            let trimmed_name = name_input.trim();
                            let trimmed_cmd = cmd_input.trim();

                            if let Err(err) = validate_alias_name(trimmed_name) {
                                *error_msg = Some(err.to_string());
                            } else if trimmed_cmd.is_empty() {
                                *error_msg = Some("Command string cannot be empty".to_string());
                            } else {
                                if *is_editing {
                                    aliases[selected] = AliasEntry {
                                        name: trimmed_name.to_string(),
                                        command: trimmed_cmd.to_string(),
                                    };
                                } else {
                                    aliases.push(AliasEntry {
                                        name: trimmed_name.to_string(),
                                        command: trimmed_cmd.to_string(),
                                    });
                                    selected = aliases.len() - 1;
                                }
                                modified = true;
                                mode = Mode::List;
                            }
                        }
                        KeyCode::Backspace => {
                            let active_str = if *active_field == 0 {
                                name_input
                            } else {
                                cmd_input
                            };
                            active_str.pop();
                            *error_msg = None;
                        }
                        KeyCode::Char(c) => {
                            let active_str = if *active_field == 0 {
                                name_input
                            } else {
                                cmd_input
                            };
                            active_str.push(c);
                            *error_msg = None;
                        }
                        _ => {}
                    },
                }
            }
        }

        drop(terminal);

        if modified {
            save_aliases_to_file(&bnbrc_path, &aliases)?;
        }

        Ok(modified)
    }
}

pub fn validate_alias_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Alias name cannot be empty");
    }
    if name.chars().all(|c| c == '.') {
        return Err("Dot-only alias names (e.g. '..') are invalid");
    }
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
            return Err("Alias name can only contain letters, numbers, '-' and '_'");
        }
    }
    Ok(())
}

fn get_bnbrc_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine user home directory")?;
    Ok(home.join(".bnbrc"))
}

fn load_aliases_from_file(path: &PathBuf) -> Result<Vec<AliasEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("alias ") {
            if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[6..eq_pos].trim().to_string();
                let mut cmd = trimmed[eq_pos + 1..].trim().to_string();
                if (cmd.starts_with('"') && cmd.ends_with('"'))
                    || (cmd.starts_with('\'') && cmd.ends_with('\''))
                {
                    cmd = cmd[1..cmd.len() - 1].to_string();
                }
                if validate_alias_name(&name).is_ok() {
                    entries.push(AliasEntry { name, command: cmd });
                }
            }
        }
    }

    Ok(entries)
}

fn save_aliases_to_file(path: &PathBuf, aliases: &[AliasEntry]) -> Result<(), String> {
    let mut lines: Vec<String> = if path.exists() {
        fs::read_to_string(path)
            .map_err(|e| e.to_string())?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    // Remove existing alias lines and alias header comments
    lines.retain(|line| {
        let trimmed = line.trim();
        !trimmed.starts_with("alias ") && !trimmed.contains("# Define persistent shortcuts")
    });

    // Strip trailing empty lines
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    lines.push("".to_string());
    lines.push(
        "# =============================================================================="
            .to_string(),
    );
    lines.push("# Define persistent shortcuts (aliases)".to_string());
    lines.push(
        "# =============================================================================="
            .to_string(),
    );

    for entry in aliases {
        lines.push(format!("alias {}=\"{}\"", entry.name, entry.command));
    }

    let output = lines.join("\n") + "\n";
    fs::write(path, output).map_err(|e| format!("Failed to save ~/.bnbrc: {}", e))
}

fn render_list_view(
    stdout: &mut io::Stdout,
    aliases: &[AliasEntry],
    selected: usize,
    box_width: usize,
    content_width: usize,
) -> Result<(), String> {
    let header_badge = format!(" [{} Aliases] ", aliases.len());
    let title_line = " ⚡ Interactive Alias Manager ".to_string();
    let title_fill =
        box_width.saturating_sub(visual_width(&title_line) + visual_width(&header_badge) + 2);

    write!(
        stdout,
        "\x1b[2J\x1b[1;1H\x1b[38;5;244m╭─\x1b[1;36m{}\x1b[0m\x1b[38;5;244m{}\x1b[1;33m{}\x1b[0m\x1b[38;5;244m─╮\r\n",
        title_line,
        "─".repeat(title_fill),
        header_badge
    )
    .map_err(|e| e.to_string())?;

    let max_visible = 10;
    let scroll_offset = if selected >= max_visible {
        selected - max_visible + 1
    } else {
        0
    };

    let visible_items = aliases.iter().skip(scroll_offset).take(max_visible);
    let mut rendered_count = 0;

    for (i, entry) in visible_items.enumerate() {
        rendered_count += 1;
        let actual_idx = scroll_offset + i;
        let is_selected = actual_idx == selected;

        let alias_str = format!("{:<12} ➔  {}", entry.name, entry.command);
        let truncated = if alias_str.len() > content_width {
            &alias_str[..content_width]
        } else {
            &alias_str
        };
        let padding = " ".repeat(content_width.saturating_sub(visual_width(truncated)));

        if is_selected {
            write!(
                stdout,
                "\x1b[38;5;244m│\x1b[0m \x1b[48;5;238m\x1b[1;37m❯ {}\x1b[0m{}\x1b[38;5;244m │\r\n",
                truncated, padding
            )
            .map_err(|e| e.to_string())?;
        } else {
            write!(
                stdout,
                "\x1b[38;5;244m│\x1b[0m   \x1b[1;32m{}\x1b[0m{}\x1b[38;5;244m │\r\n",
                truncated, padding
            )
            .map_err(|e| e.to_string())?;
        }
    }

    for _ in rendered_count..max_visible {
        write!(
            stdout,
            "\x1b[38;5;244m│\x1b[0m{}\x1b[38;5;244m│\r\n",
            " ".repeat(box_width.saturating_sub(2))
        )
        .map_err(|e| e.to_string())?;
    }

    let footer_text = " [a] Add  [e] Edit  [d] Delete  [q/Esc] Exit ";
    let footer_fill = box_width.saturating_sub(footer_text.len() + 2);
    write!(
        stdout,
        "\x1b[38;5;244m╰─\x1b[38;5;248m{}\x1b[0m\x1b[38;5;244m{}╯\r\n",
        footer_text,
        "─".repeat(footer_fill)
    )
    .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn render_form_view(
    stdout: &mut io::Stdout,
    is_editing: bool,
    name: &str,
    cmd: &str,
    active_field: usize,
    error_msg: Option<&str>,
    box_width: usize,
    content_width: usize,
) -> Result<(), String> {
    let title_line = if is_editing {
        " ✏️  Edit Alias ".to_string()
    } else {
        " ➕ Add New Alias ".to_string()
    };
    let title_fill = box_width.saturating_sub(visual_width(&title_line) + 2);

    write!(
        stdout,
        "\x1b[2J\x1b[1;1H\x1b[38;5;244m╭─\x1b[1;36m{}\x1b[0m\x1b[38;5;244m─╮\r\n",
        title_line + &"─".repeat(title_fill)
    )
    .map_err(|e| e.to_string())?;

    let name_marker = if active_field == 0 {
        "\x1b[1;33m❯ Name:   \x1b[0m"
    } else {
        "  Name:   "
    };
    let cmd_marker = if active_field == 1 {
        "\x1b[1;33m❯ Command:\x1b[0m"
    } else {
        "  Command:"
    };

    let name_pad = " ".repeat(content_width.saturating_sub(10 + name.len()));
    let cmd_pad = " ".repeat(content_width.saturating_sub(10 + cmd.len()));

    write!(
        stdout,
        "\x1b[38;5;244m│\x1b[0m {}{}{}\x1b[38;5;244m │\r\n",
        name_marker, name, name_pad
    )
    .map_err(|e| e.to_string())?;

    write!(
        stdout,
        "\x1b[38;5;244m│\x1b[0m {}{}{}\x1b[38;5;244m │\r\n",
        cmd_marker, cmd, cmd_pad
    )
    .map_err(|e| e.to_string())?;

    let err_line = error_msg.unwrap_or("");
    let err_pad = " ".repeat(content_width.saturating_sub(visual_width(err_line)));
    write!(
        stdout,
        "\x1b[38;5;244m│\x1b[0m \x1b[1;31m{}\x1b[0m{}\x1b[38;5;244m │\r\n",
        err_line, err_pad
    )
    .map_err(|e| e.to_string())?;

    let footer_text = " [Tab] Switch Field  [Enter] Save  [Esc] Cancel ";
    let footer_fill = box_width.saturating_sub(footer_text.len() + 2);
    write!(
        stdout,
        "\x1b[38;5;244m╰─\x1b[38;5;248m{}\x1b[0m\x1b[38;5;244m{}╯\r\n",
        footer_text,
        "─".repeat(footer_fill)
    )
    .map_err(|e| e.to_string())
}

fn render_delete_confirm(
    stdout: &mut io::Stdout,
    entry: &AliasEntry,
    box_width: usize,
) -> Result<(), String> {
    let title_line = " 🗑️ Delete Alias ".to_string();
    let title_fill = box_width.saturating_sub(visual_width(&title_line) + 2);

    write!(
        stdout,
        "\x1b[2J\x1b[1;1H\x1b[38;5;244m╭─\x1b[1;36m{}\x1b[0m\x1b[38;5;244m─╮\r\n",
        title_line + &"─".repeat(title_fill)
    )
    .map_err(|e| e.to_string())?;

    let msg = format!("Are you sure you want to delete '{}'?", entry.name);
    let msg_pad = " ".repeat((box_width - 4).saturating_sub(msg.len()));

    write!(
        stdout,
        "\x1b[38;5;244m│\x1b[0m \x1b[1;31m{}\x1b[0m{}\x1b[38;5;244m │\r\n",
        msg, msg_pad
    )
    .map_err(|e| e.to_string())?;

    let footer_text = " [y/Enter] Confirm Delete  [Any Key] Cancel ";
    let footer_fill = box_width.saturating_sub(footer_text.len() + 2);
    write!(
        stdout,
        "\x1b[38;5;244m╰─\x1b[38;5;248m{}\x1b[0m\x1b[38;5;244m{}╯\r\n",
        footer_text,
        "─".repeat(footer_fill)
    )
    .map_err(|e| e.to_string())
}
