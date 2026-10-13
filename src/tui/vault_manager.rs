use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType,
};
use std::collections::HashSet;
use std::io::{stdout, Write};

use crate::vault::{delete_secret, get_secret, list_secret_keys, set_secret};

enum Mode {
    Normal,
    ConfirmDelete,
    AddingKeyName { input: String },
    AddingSecretValue { key_name: String, input: String },
}

pub fn run_vault_manager() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut out = stdout();
    let _ = execute!(out, Hide);

    let res = main_loop(&mut out);

    let _ = execute!(out, Show, Clear(ClearType::FromCursorDown));
    let _ = disable_raw_mode();
    println!();
    res
}

fn main_loop(out: &mut std::io::Stdout) -> Result<(), String> {
    let mut selected_idx: usize = 0;
    let mut revealed_keys: HashSet<String> = HashSet::new();
    let mut status_msg: Option<String> = None;
    let mut mode = Mode::Normal;

    loop {
        let keys = list_secret_keys();
        if selected_idx >= keys.len() && !keys.is_empty() {
            selected_idx = keys.len() - 1;
        }

        render(out, &keys, selected_idx, &revealed_keys, &status_msg, &mode)?;

        if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
            match &mut mode {
                Mode::Normal => match (code, modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL)
                    | (KeyCode::Char('q'), _)
                    | (KeyCode::Esc, _) => break,

                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        selected_idx = selected_idx.saturating_sub(1);
                        status_msg = None;
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        if !keys.is_empty() && selected_idx + 1 < keys.len() {
                            selected_idx += 1;
                        }
                        status_msg = None;
                    }
                    (KeyCode::Char(' '), _) => {
                        if let Some(key) = keys.get(selected_idx) {
                            if revealed_keys.contains(key) {
                                revealed_keys.remove(key);
                            } else {
                                revealed_keys.insert(key.clone());
                            }
                        }
                    }
                    (KeyCode::Char('d'), _) => {
                        if !keys.is_empty() {
                            mode = Mode::ConfirmDelete;
                        }
                    }
                    (KeyCode::Char('a'), _) => {
                        mode = Mode::AddingKeyName {
                            input: String::new(),
                        };
                        status_msg = None;
                    }
                    _ => {}
                },

                Mode::ConfirmDelete => match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(key) = keys.get(selected_idx) {
                            revealed_keys.remove(key);
                            let _ = delete_secret(key);
                            status_msg = Some(format!("✓ Removed secret '{}'", key));
                        }
                        mode = Mode::Normal;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        status_msg = Some("Deletion cancelled.".to_string());
                        mode = Mode::Normal;
                    }
                    _ => {}
                },

                Mode::AddingKeyName { input } => match code {
                    KeyCode::Enter => {
                        let trimmed = input.trim().to_string();
                        if trimmed.is_empty() {
                            status_msg = Some("Key name cannot be empty.".to_string());
                            mode = Mode::Normal;
                        } else if !trimmed
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                        {
                            status_msg = Some("Keys must only contain [a-zA-Z0-9_-]".to_string());
                            mode = Mode::Normal;
                        } else {
                            mode = Mode::AddingSecretValue {
                                key_name: trimmed,
                                input: String::new(),
                            };
                        }
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                        input.push(c);
                    }
                    KeyCode::Esc => {
                        mode = Mode::Normal;
                        status_msg = Some("Action cancelled.".to_string());
                    }
                    _ => {}
                },

                Mode::AddingSecretValue { key_name, input } => match code {
                    KeyCode::Enter => {
                        let trimmed_val = input.trim();
                        if trimmed_val.is_empty() {
                            status_msg = Some("Secret value cannot be empty.".to_string());
                        } else {
                            let _ = set_secret(key_name, trimmed_val);
                            status_msg = Some(format!("✓ Secret '{}' securely stored.", key_name));
                        }
                        mode = Mode::Normal;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                        input.push(c);
                    }
                    KeyCode::Esc => {
                        mode = Mode::Normal;
                        status_msg = Some("Action cancelled.".to_string());
                    }
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

fn render(
    out: &mut std::io::Stdout,
    keys: &[String],
    selected_idx: usize,
    revealed: &HashSet<String>,
    status_msg: &Option<String>,
    mode: &Mode,
) -> Result<(), String> {
    let _ = execute!(out, Clear(ClearType::All), MoveTo(0, 0));

    // Header
    writeln!(
        out,
        "\r\x1b[1;36m🔐 bnb Secret Vault Manager\x1b[0m \x1b[38;5;244m({} stored)\x1b[0m",
        keys.len()
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "\r\x1b[38;5;240m─────────────────────────────────────────────────────────────────\x1b[0m"
    )
    .map_err(|e| e.to_string())?;

    // Secret list
    if keys.is_empty() {
        writeln!(
            out,
            "\r  \x1b[38;5;244m(No secrets in vault. Press \x1b[1;32m[a]\x1b[38;5;244m to add one.)\x1b[0m"
        )
        .map_err(|e| e.to_string())?;
    } else {
        for (i, key) in keys.iter().enumerate() {
            let is_sel = i == selected_idx;
            let marker = if is_sel { "\x1b[1;36m❯\x1b[0m " } else { "  " };

            let value_display = if revealed.contains(key) {
                let actual = get_secret(key).unwrap_or_else(|| "<error>".to_string());
                format!("\x1b[1;32m[{}]\x1b[0m", actual)
            } else {
                "\x1b[38;5;244m[••••••••]\x1b[0m".to_string()
            };

            if is_sel {
                writeln!(
                    out,
                    "\r{}\x1b[48;5;236m\x1b[1;37m{:<24}\x1b[0m {}",
                    marker,
                    key,
                    value_display
                )
                .map_err(|e| e.to_string())?;
            } else {
                writeln!(out, "\r{}{:<24} {}", marker, key, value_display)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // Modal Status / Input Prompts
    writeln!(
        out,
        "\r\x1b[38;5;240m─────────────────────────────────────────────────────────────────\x1b[0m"
    )
    .map_err(|e| e.to_string())?;

    match mode {
        Mode::Normal => {
            if let Some(msg) = status_msg {
                writeln!(out, "\r  \x1b[1;33m{}\x1b[0m", msg).map_err(|e| e.to_string())?;
            } else {
                writeln!(
                    out,
                    "\r  \x1b[38;5;244m[↑/↓ or j/k] Navigate  •  [Space] Reveal  •  [a] Add  •  [d] Delete  •  [q] Exit\x1b[0m"
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Mode::ConfirmDelete => {
            let key = &keys[selected_idx];
            writeln!(
                out,
                "\r  \x1b[1;31mDelete secret '{}'? Press [y] to confirm, [n/Esc] to cancel.\x1b[0m",
                key
            )
            .map_err(|e| e.to_string())?;
        }
        Mode::AddingKeyName { input } => {
            writeln!(
                out,
                "\r  \x1b[1;36mKey Name: \x1b[1;37m{}\x1b[1;32m█\x1b[0m \x1b[38;5;244m([Enter] Next, [Esc] Cancel)\x1b[0m",
                input
            )
            .map_err(|e| e.to_string())?;
        }
        Mode::AddingSecretValue { key_name, input } => {
            let masked: String = "*".repeat(input.len());
            writeln!(
                out,
                "\r  \x1b[1;36mValue for '{}': \x1b[1;37m{}\x1b[1;32m█\x1b[0m \x1b[38;5;244m([Enter] Save, [Esc] Cancel)\x1b[0m",
                key_name, masked
            )
            .map_err(|e| e.to_string())?;
        }
    }

    out.flush().map_err(|e| e.to_string())
}