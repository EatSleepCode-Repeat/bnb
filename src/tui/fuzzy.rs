use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::fs;
use std::io::{self, stdout, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct FuzzyFinder;

struct TerminalModeGuard {
    alternate_screen: bool,
}

impl TerminalModeGuard {
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

impl Drop for TerminalModeGuard {
    fn drop(&mut self) {
        if self.alternate_screen {
            let _ = stdout().execute(LeaveAlternateScreen);
        }
        let _ = disable_raw_mode();
    }
}

impl FuzzyFinder {
    pub fn select(title: &str, items: &[String]) -> Result<Option<String>, String> {
        let selections = Self::select_multi(title, items)?;
        Ok(selections.into_iter().next())
    }

    pub fn select_multi(title: &str, items: &[String]) -> Result<Vec<String>, String> {
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let mut terminal = TerminalModeGuard::enter()
            .map_err(|error| format!("fuzzy finder: cannot enable raw mode: {}", error))?;
        let mut stdout = stdout();
        terminal
            .enter_alternate_screen(&mut stdout)
            .map_err(|error| format!("fuzzy finder: cannot enter alternate screen: {}", error))?;

        let matcher = SkimMatcherV2::default();
        let mut query = String::new();
        let mut selected = 0;
        let mut selected_set: HashSet<String> = HashSet::new();
        let mut result_selections = Vec::new();

        loop {
            let mut matches: Vec<(&String, i64, Vec<usize>)> = items
                .iter()
                .filter_map(|item| {
                    if query.is_empty() {
                        Some((item, 0, Vec::new()))
                    } else {
                        matcher
                            .fuzzy_indices(item, &query)
                            .map(|(score, indices)| (item, score, indices))
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
            let box_width = (term_cols as usize).saturating_sub(4).clamp(48, 76);
            let content_width = box_width.saturating_sub(4);

            let selected_count = selected_set.len();
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

            let title_line = format!(" 🔍 {} ", title);
            let title_fill = box_width
                .saturating_sub(visual_width(&title_line) + visual_width(&header_badge) + 2);

            write!(
                stdout,
                "\x1b[2J\x1b[1;1H\x1b[38;5;244m╭─\x1b[1;36m{}\x1b[0m\x1b[38;5;244m{}\x1b[1;33m{}\x1b[0m\x1b[38;5;244m─╮\r\n",
                title_line,
                "─".repeat(title_fill),
                header_badge
            )
            .map_err(|e| format!("fuzzy finder: terminal write failed: {}", e))?;

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
            .map_err(|e| format!("fuzzy finder: terminal write failed: {}", e))?;

            write!(
                stdout,
                "\x1b[38;5;244m├{}\x1b[0m\r\n",
                "─".repeat(box_width.saturating_sub(2))
            )
            .map_err(|e| format!("fuzzy finder: terminal write failed: {}", e))?;

            let visible_items = matches.iter().skip(scroll_offset).take(max_visible);
            for (i, (item, _, indices)) in visible_items.enumerate() {
                let actual_index = scroll_offset + i;
                let is_selected = actual_index == selected;
                let is_checked = selected_set.contains(*item);

                let rendered_line =
                    render_item_line(item, indices, is_selected, is_checked, content_width);

                write!(
                    stdout,
                    "\x1b[38;5;244m│\x1b[0m {}\x1b[38;5;244m │\r\n",
                    rendered_line
                )
                .map_err(|e| format!("fuzzy finder: terminal write failed: {}", e))?;
            }

            let rendered_count = matches.iter().skip(scroll_offset).take(max_visible).count();
            for _ in rendered_count..max_visible {
                write!(
                    stdout,
                    "\x1b[38;5;244m│\x1b[0m{}\x1b[38;5;244m│\r\n",
                    " ".repeat(box_width.saturating_sub(2))
                )
                .map_err(|e| format!("fuzzy finder: terminal write failed: {}", e))?;
            }

            let footer_text = " [Enter] Confirm  [Tab/Space] Toggle  [Esc] Cancel ";
            let footer_fill = box_width.saturating_sub(footer_text.len() + 2);
            write!(
                stdout,
                "\x1b[38;5;244m╰─\x1b[38;5;248m{}\x1b[0m\x1b[38;5;244m{}╯\r\n",
                footer_text,
                "─".repeat(footer_fill)
            )
            .map_err(|e| format!("fuzzy finder: terminal write failed: {}", e))?;

            stdout
                .flush()
                .map_err(|e| format!("fuzzy finder: terminal flush failed: {}", e))?;

            let event =
                event::read().map_err(|e| format!("fuzzy finder: terminal input failed: {}", e))?;
            if let Event::Key(key) = event {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => break,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Enter, _) => {
                        if selected_set.is_empty() {
                            if !matches.is_empty() && selected < matches.len() {
                                result_selections.push(matches[selected].0.clone());
                            }
                        } else {
                            for item in items {
                                if selected_set.contains(item) {
                                    result_selections.push(item.clone());
                                }
                            }
                        }
                        break;
                    }
                    (KeyCode::Tab, _) | (KeyCode::Char(' '), KeyModifiers::NONE) => {
                        if !matches.is_empty() && selected < matches.len() {
                            let item = matches[selected].0.clone();
                            if selected_set.contains(&item) {
                                selected_set.remove(&item);
                            } else {
                                selected_set.insert(item);
                            }
                            if selected + 1 < matches.len() {
                                selected += 1;
                            }
                        }
                    }
                    (KeyCode::Up, _) => selected = selected.saturating_sub(1),
                    (KeyCode::Down, _) => {
                        if selected + 1 < matches.len() {
                            selected += 1;
                        }
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

        drop(terminal);
        Ok(result_selections)
    }
}

pub fn visual_width(s: &str) -> usize {
    let mut w = 0;
    for c in s.chars() {
        if c == '\u{fe0f}' {
            continue;
        }
        if ('\u{1F300}'..='\u{1F9FF}').contains(&c)
            || ('\u{2600}'..='\u{26FF}').contains(&c)
            || ('\u{2700}'..='\u{27BF}').contains(&c)
            || ('\u{1F000}'..='\u{1F02F}').contains(&c)
            || ('\u{1F0A0}'..='\u{1F0FF}').contains(&c)
            || ('\u{1F100}'..='\u{1F6FF}').contains(&c)
            || ('\u{2300}'..='\u{23FF}').contains(&c)
        {
            w += 2;
        } else {
            w += 1;
        }
    }
    w
}

fn render_item_line(
    item: &str,
    indices: &[usize],
    is_selected: bool,
    is_checked: bool,
    max_width: usize,
) -> String {
    let icon = get_file_icon(item);
    let prefix = if is_selected { "❯ " } else { "  " };
    let check_mark_text = if is_checked { "[✓] " } else { "    " };

    let prefix_width = visual_width(prefix) + visual_width(check_mark_text) + visual_width(icon);
    let avail_text_len = max_width.saturating_sub(prefix_width);

    let (display_item, offset) = if item.len() > avail_text_len && avail_text_len > 0 {
        let start = item.len().saturating_sub(avail_text_len);
        (&item[start..], start)
    } else {
        (item, 0)
    };

    let mut highlighted = String::new();
    for (i, ch) in display_item.chars().enumerate() {
        let original_idx = offset + i;
        if indices.contains(&original_idx) {
            if is_selected {
                highlighted.push_str("\x1b[1;33m");
            } else {
                highlighted.push_str("\x1b[1;36m");
            }
            highlighted.push(ch);
            if is_selected {
                highlighted.push_str("\x1b[22m\x1b[37m");
            } else {
                highlighted.push_str("\x1b[0m\x1b[38;5;250m");
            }
        } else {
            highlighted.push(ch);
        }
    }

    let line_content_width = prefix_width + visual_width(display_item);
    let padding_needed = max_width.saturating_sub(line_content_width);
    let padding = " ".repeat(padding_needed);

    if is_selected {
        format!(
            "\x1b[48;5;238m\x1b[1;37m{}{}{}{}{}\x1b[0m",
            prefix, check_mark_text, icon, highlighted, padding
        )
    } else {
        let check_colored = if is_checked {
            "\x1b[1;32m[✓]\x1b[0m "
        } else {
            "    "
        };
        format!(
            "{}{}{}\x1b[38;5;250m{}\x1b[0m{}",
            prefix, check_colored, icon, highlighted, padding
        )
    }
}

fn get_file_icon(path: &str) -> &'static str {
    if path.contains("History Search")
        || path.contains("Fuzzy File Finder")
        || path.contains("Git Branch Switcher")
        || path.contains("Directory Jumper")
        || path.contains("Global Project Switcher")
        || path.contains("Git Status Stager")
        || path.contains("Configure Prompt Theme")
        || path.contains("Visual Trash Vault")
        || path.contains("Reload Shell Config")
        || path.contains("Clear Terminal Screen")
        || path.contains("Exit Shell Session")
    {
        ""
    } else if path.starts_with("[Staged]") {
        "✅ "
    } else if path.starts_with("[Unstaged]") {
        "📝 "
    } else if path.starts_with("[Untracked]") {
        "❓ "
    } else if path.starts_with("origin/") || path.starts_with("upstream/") {
        "🌐 "
    } else if Path::new(path).is_dir() || path.starts_with("~/") {
        "📁 "
    } else if path.ends_with(".rs") {
        "🦀 "
    } else if path.ends_with(".toml") {
        "⚙️ "
    } else if path.ends_with(".md") {
        "📝 "
    } else if path.ends_with(".json") || path.ends_with(".yaml") || path.ends_with(".yml") {
        "📋 "
    } else if path.ends_with(".sh") || path.ends_with(".bash") || path.ends_with(".zsh") {
        "⚡ "
    } else if path.ends_with(".lock") {
        "🔒 "
    } else if path.contains("LICENSE") {
        "📜 "
    } else if path.starts_with('.') {
        "🔧 "
    } else if path.contains('/') {
        "📄 "
    } else {
        "🌿 "
    }
}

pub fn collect_files(max_files: usize) -> Vec<String> {
    let mut files = Vec::new();
    let mut stack = vec![PathBuf::from(".")];
    let ignored_dirs = [
        ".git",
        "node_modules",
        "target",
        ".bnb",
        "dist",
        "build",
        ".venv",
        "venv",
        ".next",
        ".cache",
    ];

    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if !ignored_dirs.contains(&name.as_str()) && !name.starts_with('.') {
                        stack.push(path);
                    }
                } else {
                    let display_path = if let Ok(rel) = path.strip_prefix(".") {
                        rel.display().to_string()
                    } else {
                        path.display().to_string()
                    };

                    files.push(display_path);

                    if files.len() >= max_files {
                        return files;
                    }
                }
            }
        }
    }

    files.sort();
    files
}

pub fn collect_git_branches() -> Vec<String> {
    let output = Command::new("git")
        .args(["branch", "-a", "--format=%(HEAD)|%(refname:short)"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();
    let mut head_branch = None;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.is_empty() {
            continue;
        }
        let is_head = parts[0].trim() == "*";
        let name = parts.get(1).unwrap_or(&"").trim();

        if name.is_empty() || name.contains("HEAD detached") {
            continue;
        }

        if is_head {
            head_branch = Some(name.to_string());
        } else {
            branches.push(name.to_string());
        }
    }

    if let Some(head) = head_branch {
        branches.insert(0, head);
    }

    branches
}

pub fn collect_z_directories() -> Vec<String> {
    crate::builtins::z::get_ranked_directories()
}

pub fn collect_projects() -> Vec<String> {
    let home = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Vec::new(),
    };

    let mut projects = Vec::new();
    let mut stack = vec![(home.clone(), 0)];
    let ignored_dirs = [
        "Library",
        "Downloads",
        ".cargo",
        ".rustup",
        "node_modules",
        "target",
        ".bnb",
        "dist",
        "build",
        ".venv",
        "venv",
        ".cache",
        ".Trash",
    ];

    while let Some((dir, depth)) = stack.pop() {
        if depth > 4 {
            continue;
        }

        if dir.join(".git").exists() && dir != home {
            let path_str = dir.display().to_string();
            let display_path = if path_str.starts_with(&home.display().to_string()) {
                path_str.replacen(&home.display().to_string(), "~", 1)
            } else {
                path_str
            };
            projects.push(display_path);
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !ignored_dirs.contains(&name.as_str()) && !name.starts_with('.') {
                        stack.push((entry.path(), depth + 1));
                    }
                }
            }
        }
    }

    projects.sort();
    projects
}

pub fn collect_git_status_items() -> Vec<String> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let file_path = line[3..].trim().to_string();

        if index_status != ' ' && index_status != '?' {
            items.push(format!("[Staged] {}", file_path));
        }
        if worktree_status != ' ' && worktree_status != '?' {
            items.push(format!("[Unstaged] {}", file_path));
        }
        if index_status == '?' && worktree_status == '?' {
            items.push(format!("[Untracked] {}", file_path));
        }
    }

    items
}
