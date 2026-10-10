use crate::tui::fuzzy::visual_width;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use std::fs::{self, File};
use std::io::{self, stdout, Read, Write};
use std::path::{Path, PathBuf};

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

struct Entry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    size: u64,
}

pub enum Selection {
    Directory(PathBuf),
    File(PathBuf),
}

pub fn run_directory_browser() -> Result<Option<Selection>, String> {
    let mut current_dir =
        std::env::current_dir().map_err(|error| format!("directory browser: {}", error))?;
    let mut selected = 0;
    let mut terminal = TerminalModeGuard::enter()
        .map_err(|error| format!("directory browser: cannot enable raw mode: {}", error))?;
    let mut stdout = stdout();
    terminal
        .enter_alternate_screen(&mut stdout)
        .map_err(|error| {
            format!(
                "directory browser: cannot enter alternate screen: {}",
                error
            )
        })?;

    loop {
        let entries = read_entries(&current_dir)?;
        if selected >= entries.len() {
            selected = entries.len().saturating_sub(1);
        }
        render(&mut stdout, &current_dir, &entries, selected)?;

        if let Event::Key(key) =
            event::read().map_err(|error| format!("directory browser: input failed: {}", error))?
        {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => return Ok(None),
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(None),
                (KeyCode::Up, _) => selected = selected.saturating_sub(1),
                (KeyCode::Down, _) => {
                    if selected + 1 < entries.len() {
                        selected += 1;
                    }
                }
                (KeyCode::Backspace, _) | (KeyCode::Left, _) => {
                    if let Some(parent) = current_dir.parent() {
                        current_dir = parent.to_path_buf();
                        selected = 0;
                    }
                }
                (KeyCode::Char(' '), KeyModifiers::NONE) => {
                    return Ok(Some(Selection::Directory(current_dir)));
                }
                (KeyCode::Enter, _) => {
                    if let Some(entry) = entries.get(selected) {
                        if entry.is_dir {
                            current_dir = entry.path.clone();
                            selected = 0;
                        } else {
                            return Ok(Some(Selection::File(entry.path.clone())));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn read_entries(directory: &Path) -> Result<Vec<Entry>, String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "directory browser: cannot read {}: {}",
            directory.display(),
            error
        )
    })?;
    let mut result = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("directory browser: {}", error))?;
        let metadata = entry
            .metadata()
            .map_err(|error| format!("directory browser: {}: {}", entry.path().display(), error))?;
        result.push(Entry {
            path: entry.path(),
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    result.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

fn render(
    stdout: &mut io::Stdout,
    current_dir: &Path,
    entries: &[Entry],
    selected: usize,
) -> Result<(), String> {
    let (columns, rows) = size().unwrap_or((80, 24));
    let width = columns as usize;
    let height = rows as usize;
    write!(stdout, "\x1b[2J\x1b[1;1H").map_err(|error| error.to_string())?;

    if width < 50 || height < 10 {
        write_line(
            stdout,
            format_args!("Terminal too small for directory browser (need 50x10)."),
        )?;
        stdout.flush().map_err(|error| error.to_string())?;
        return Ok(());
    }

    write_line(
        stdout,
        format_args!("\x1b[1;36m📁 Directory Overview\x1b[0m"),
    )?;
    write_line(
        stdout,
        format_args!(
            "\x1b[38;5;244m{}\x1b[0m",
            truncate(&safe_text(&current_dir.display().to_string()), width)
        ),
    )?;

    let list_width = (width / 2).max(24);
    let preview_width = width.saturating_sub(list_width + 3);
    let content_rows = height.saturating_sub(5);
    let max_entries = content_rows.saturating_sub(1);
    let scroll_offset = if selected >= max_entries {
        selected - max_entries + 1
    } else {
        0
    };
    let end = entries.len().min(scroll_offset + max_entries);

    let left_width = list_width.saturating_sub(2);
    let right_width = preview_width.saturating_sub(1);
    let header_left = pad_right("Name", left_width);
    write_line(
        stdout,
        format_args!(
            "\x1b[38;5;244m{} │ {}\x1b[0m",
            header_left,
            truncate("Preview", right_width)
        ),
    )?;

    let mut preview = preview_lines(entries.get(selected), max_entries)?;
    preview.resize(max_entries, String::new());
    for (offset, index) in (scroll_offset..end).enumerate() {
        let entry = &entries[index];
        let icon = if entry.is_dir { "📁" } else { "📄" };
        let marker = if index == selected { "❯" } else { " " };
        let label = truncate(
            &format!("{} {} {}", marker, icon, safe_text(&entry.name)),
            left_width,
        );
        let label = pad_right(&label, left_width);
        let preview_line = truncate(&preview[offset], right_width);
        if index == selected {
            write_line(
                stdout,
                format_args!(
                    "\x1b[48;5;238m\x1b[1;37m{}\x1b[0m \x1b[38;5;244m│\x1b[0m {}",
                    label, preview_line
                ),
            )
        } else {
            write_line(
                stdout,
                format_args!("{} \x1b[38;5;244m│\x1b[0m {}", label, preview_line),
            )
        }?;
    }
    for offset in end.saturating_sub(scroll_offset)..max_entries {
        write_line(
            stdout,
            format_args!(
                "{} \x1b[38;5;244m│\x1b[0m {}",
                " ".repeat(left_width),
                truncate(&preview[offset], right_width)
            ),
        )?;
    }
    let footer =
        "[↑/↓] Select  [Enter] Open/insert  [←/Backspace] Parent  [Space] Choose folder  [Esc] Cancel";
    write_line(
        stdout,
        format_args!("\x1b[38;5;244m{}\x1b[0m", truncate(footer, width)),
    )?;
    stdout.flush().map_err(|error| error.to_string())
}

fn write_line(stdout: &mut io::Stdout, args: std::fmt::Arguments<'_>) -> Result<(), String> {
    stdout
        .write_fmt(args)
        .and_then(|()| stdout.write_all(b"\r\n"))
        .map_err(|error| error.to_string())
}

fn preview_lines(entry: Option<&Entry>, max_rows: usize) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    let Some(entry) = entry else {
        return Ok(lines);
    };

    if entry.is_dir {
        let children = read_entries(&entry.path)?;
        lines.push(format!("{} item(s)", children.len()));
        for child in children.iter().take(max_rows.saturating_sub(1)) {
            let icon = if child.is_dir { "📁" } else { "📄" };
            lines.push(format!("{} {}", icon, safe_text(&child.name)));
        }
    } else {
        lines.push(format!("{} bytes", entry.size));
        let mut file = File::open(&entry.path).map_err(|error| {
            format!(
                "directory browser: cannot preview {}: {}",
                entry.path.display(),
                error
            )
        })?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(8192)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                format!(
                    "directory browser: cannot preview {}: {}",
                    entry.path.display(),
                    error
                )
            })?;
        if bytes.iter().any(|byte| *byte == 0) {
            lines.push("Binary file".to_string());
        } else {
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines().take(max_rows.saturating_sub(1)) {
                lines.push(safe_text(line));
            }
        }
    }

    Ok(lines)
}

fn truncate(text: &str, max_chars: usize) -> String {
    if visual_width(text) <= max_chars {
        text.to_string()
    } else if max_chars <= 1 {
        "…".to_string()
    } else {
        let mut result = String::new();
        for character in text.chars() {
            if visual_width(&result) + visual_width(&character.to_string()) > max_chars - 1 {
                break;
            }
            result.push(character);
        }
        result.push('…');
        result
    }
}

fn safe_text(text: &str) -> String {
    let mut result = String::new();
    for character in text.chars() {
        if character.is_control() {
            result.push_str(&format!("\\u{{{:x}}}", character as u32));
        } else {
            result.push(character);
        }
    }
    result
}

fn pad_right(text: &str, width: usize) -> String {
    format!(
        "{}{}",
        text,
        " ".repeat(width.saturating_sub(visual_width(text)))
    )
}

pub fn quote_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    let mut escaped = String::new();
    for character in path.chars() {
        if matches!(character, '"' | '\\' | '$' | '`') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::{quote_path, safe_text};
    use std::path::Path;

    #[test]
    fn quotes_paths_for_shell_insertion() {
        assert_eq!(
            quote_path(Path::new("/tmp/a b/$file\"name")),
            "\"/tmp/a b/\\$file\\\"name\""
        );
    }

    #[test]
    fn escapes_control_characters_in_terminal_text() {
        assert_eq!(safe_text("name\u{1b}[31m"), "name\\u{1b}[31m");
    }
}
