use crate::safety::trash::{
    clear_trash, list_trash_entries, purge_entry, restore_entry, TrashEntry,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::io::{self, stdout, Write};
use std::time::{SystemTime, UNIX_EPOCH};

struct TerminalModeGuard;

impl TerminalModeGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalModeGuard {
    fn drop(&mut self) {
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn run_trash_vault() -> Result<(), String> {
    let mut entries = list_trash_entries();
    if entries.is_empty() {
        println!("\x1b[1;32m🗑️  Trash Vault is empty!\x1b[0m");
        return Ok(());
    }

    let _guard = TerminalModeGuard::enter()
        .map_err(|e| format!("trash-vault: cannot enter raw mode: {}", e))?;
    let mut stdout = stdout();

    let matcher = SkimMatcherV2::default();
    let mut query = String::new();
    let mut selected: usize = 0;
    let mut selected_ids: HashSet<String> = HashSet::new();
    let mut restored_paths = Vec::new();

    loop {
        entries = list_trash_entries();
        if entries.is_empty() {
            break;
        }

        let mut matches: Vec<(&TrashEntry, i64, Vec<usize>)> = entries
            .iter()
            .filter_map(|entry| {
                let path_str = entry.original_path.display().to_string();
                if query.is_empty() {
                    Some((entry, 0, Vec::new()))
                } else {
                    matcher
                        .fuzzy_indices(&path_str, &query)
                        .map(|(score, indices)| (entry, score, indices))
                }
            })
            .collect();

        if !query.is_empty() {
            matches.sort_by_key(|a| std::cmp::Reverse(a.1));
        }

        if selected >= matches.len() {
            selected = matches.len().saturating_sub(1);
        }

        let max_visible = 12;
        let total_matches = matches.len();

        let scroll_offset = if selected >= max_visible {
            selected - max_visible + 1
        } else {
            0
        };

        let (term_cols, _) = size().unwrap_or((80, 24));
        let box_width = (term_cols as usize).saturating_sub(4).clamp(52, 80);
        let content_width = box_width.saturating_sub(4);

        let selected_count = selected_ids.len();
        let header_badge = if selected_count > 0 {
            format!(
                " [{}/{} items | {} selected] ",
                if total_matches == 0 { 0 } else { selected + 1 },
                total_matches,
                selected_count
            )
        } else {
            format!(
                " [{}/{} items] ",
                if total_matches == 0 { 0 } else { selected + 1 },
                total_matches
            )
        };

        let title_line = " 🗑️ Visual Trash Vault ";
        let title_fill = box_width
            .saturating_sub(title_line.len() + header_badge.len() + 2);

        write!(
            stdout,
            "\x1b[2J\x1b[1;1H\x1b[38;5;244m╭{}\x1b[1;36m{}\x1b[0m\x1b[38;5;244m{}\x1b[1;33m{}\x1b[0m\x1b[38;5;244m{}╮\r\n",
            "─",
            title_line,
            "─".repeat(title_fill),
            header_badge,
            "─"
        )
        .map_err(|e| e.to_string())?;

        let truncated_query = if query.len() > content_width.saturating_sub(4) {
            &query[query.len() - (content_width.saturating_sub(4))..]
        } else {
            &query
        };
        let query_padding = " ".repeat(content_width.saturating_sub(truncated_query.len() + 3));

        write!(
            stdout,
            "\x1b[38;5;244m│ \x1b[1;32m❯ \x1b[1;37m{}\x1b[0m\x1b[47m\x1b[30m \x1b[0m{}\x1b[38;5;244m │\r\n",
            truncated_query, query_padding
        )
        .map_err(|e| e.to_string())?;

        write!(
            stdout,
            "\x1b[38;5;244m├{}\x1b[0m\r\n",
            "─".repeat(box_width.saturating_sub(2))
        )
        .map_err(|e| e.to_string())?;

        let visible_items = matches.iter().skip(scroll_offset).take(max_visible);
        for (i, (entry, _, _)) in visible_items.enumerate() {
            let actual_index = scroll_offset + i;
            let is_selected = actual_index == selected;
            let is_checked = selected_ids.contains(&entry.id);

            let line = render_trash_line(entry, is_selected, is_checked, content_width);
            write!(stdout, "\x1b[38;5;244m│\x1b[0m {}\x1b[38;5;244m │\r\n", line)
                .map_err(|e| e.to_string())?;
        }

        let rendered_count = matches.iter().skip(scroll_offset).take(max_visible).count();
        for _ in rendered_count..max_visible {
            write!(
                stdout,
                "\x1b[38;5;244m│\x1b[0m{}\x1b[38;5;244m│\r\n",
                " ".repeat(box_width.saturating_sub(2))
            )
            .map_err(|e| e.to_string())?;
        }

        let footer_text = " [Enter/r] Restore  [d] Purge  [c] Clear All  [Tab/Space] Select  [Esc] Exit ";
        let footer_fill = box_width.saturating_sub(footer_text.len() + 2);
        write!(
            stdout,
            "\x1b[38;5;244m╰{}\x1b[38;5;248m{}\x1b[0m\x1b[38;5;244m{}╯\r\n",
            "─",
            footer_text,
            "─".repeat(footer_fill)
        )
        .map_err(|e| e.to_string())?;

        stdout.flush().map_err(|e| e.to_string())?;

        if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('q'), _) => break,
                (KeyCode::Up, _) => selected = selected.saturating_sub(1),
                (KeyCode::Down, _) => {
                    if selected + 1 < matches.len() {
                        selected += 1;
                    }
                }
                (KeyCode::Tab, _) | (KeyCode::Char(' '), KeyModifiers::NONE) => {
                    if !matches.is_empty() && selected < matches.len() {
                        let id = matches[selected].0.id.clone();
                        if selected_ids.contains(&id) {
                            selected_ids.remove(&id);
                        } else {
                            selected_ids.insert(id);
                        }
                        if selected + 1 < matches.len() {
                            selected += 1;
                        }
                    }
                }
                (KeyCode::Enter, _) | (KeyCode::Char('r'), KeyModifiers::NONE) => {
                    let to_restore: Vec<TrashEntry> = if selected_ids.is_empty() {
                        if !matches.is_empty() && selected < matches.len() {
                            vec![matches[selected].0.clone()]
                        } else {
                            Vec::new()
                        }
                    } else {
                        entries
                            .iter()
                            .filter(|e| selected_ids.contains(&e.id))
                            .cloned()
                            .collect()
                    };

                    for entry in to_restore {
                        match restore_entry(&entry) {
                            Ok(_) => restored_paths.push(entry.original_path.display().to_string()),
                            Err(e) => eprintln!("\r\n{}", e),
                        }
                    }
                    selected_ids.clear();
                    break;
                }
                (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _) => {
                    let to_purge: Vec<TrashEntry> = if selected_ids.is_empty() {
                        if !matches.is_empty() && selected < matches.len() {
                            vec![matches[selected].0.clone()]
                        } else {
                            Vec::new()
                        }
                    } else {
                        entries
                            .iter()
                            .filter(|e| selected_ids.contains(&e.id))
                            .cloned()
                            .collect()
                    };

                    for entry in to_purge {
                        let _ = purge_entry(&entry);
                    }
                    selected_ids.clear();
                    selected = 0;
                }
                (KeyCode::Char('c'), KeyModifiers::NONE) => {
                    let _ = clear_trash();
                    selected_ids.clear();
                    break;
                }
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), KeyModifiers::NONE)
                | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            }
        }
    }

    drop(_guard);

    if !restored_paths.is_empty() {
        println!(
            "\x1b[1;32m✨ Restored {} item(s):\x1b[0m",
            restored_paths.len()
        );
        for p in restored_paths {
            println!("   \x1b[36m• {}\x1b[0m", p);
        }
    }

    Ok(())
}

fn render_trash_line(
    entry: &TrashEntry,
    is_selected: bool,
    is_checked: bool,
    max_width: usize,
) -> String {
    let icon = if entry.is_dir { "📁 " } else { "📄 " };
    let prefix = if is_selected { "❯ " } else { "  " };

    let time_str = format_time_ago(entry.deleted_at);
    let size_str = format_size(entry.size_bytes);
    let meta_str = format!(" ({} · {})", size_str, time_str);

    let prefix_len = 2 + 4 + 2;
    let avail_text_len = max_width.saturating_sub(prefix_len + meta_str.len());

    let raw_path = entry.original_path.display().to_string();
    let display_path = if raw_path.len() > avail_text_len {
        let start = raw_path.len() - avail_text_len;
        format!("…{}", &raw_path[start + 1..])
    } else {
        raw_path
    };

    let padding_needed =
        max_width.saturating_sub(prefix_len + display_path.len() + meta_str.len());
    let padding = " ".repeat(padding_needed);

    let check_mark = if is_checked {
        "\x1b[1;32m[✓]\x1b[0m "
    } else {
        "    "
    };

    if is_selected {
        format!(
            "\x1b[48;5;238m\x1b[1;37m{}{}{}{}\x1b[38;5;245m{}\x1b[1;37m{}\x1b[0m",
            prefix,
            if is_checked { "[✓] " } else { "    " },
            icon,
            display_path,
            meta_str,
            padding
        )
    } else {
        format!(
            "{}{}{}{}\x1b[38;5;244m{}\x1b[0m{}",
            prefix, check_mark, icon, display_path, meta_str, padding
        )
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_time_ago(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let diff = now.saturating_sub(timestamp);
    if diff >= 86400 {
        format!("{}d ago", diff / 86400)
    } else if diff >= 3600 {
        format!("{}h ago", diff / 3600)
    } else if diff >= 60 {
        format!("{}m ago", diff / 60)
    } else {
        "just now".to_string()
    }
}