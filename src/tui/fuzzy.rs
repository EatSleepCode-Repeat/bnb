use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::io::{self, stdout, Write};

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
        if items.is_empty() {
            return Ok(None);
        }

        let mut terminal = TerminalModeGuard::enter()
            .map_err(|error| format!("history search: cannot enable raw mode: {}", error))?;
        let mut stdout = stdout();
        terminal
            .enter_alternate_screen(&mut stdout)
            .map_err(|error| format!("history search: cannot enter alternate screen: {}", error))?;

        let matcher = SkimMatcherV2::default();
        let mut query = String::new();
        let mut selected = 0;
        let mut selection = None;

        loop {
            let mut matches: Vec<(&String, i64)> = items
                .iter()
                .filter_map(|item| matcher.fuzzy_match(item, &query).map(|score| (item, score)))
                .collect();
            matches.sort_by_key(|a| std::cmp::Reverse(a.1));

            writeln!(
                stdout,
                "\x1b[2J\x1b[1;1H\x1b[1;34m🔍 {} (Esc to cancel, Enter to pick)\x1b[0m",
                title
            )
            .map_err(|error| format!("history search: terminal write failed: {}", error))?;
            writeln!(stdout, "Query: \x1b[33m{}\x1b[0m█", query)
                .map_err(|error| format!("history search: terminal write failed: {}", error))?;
            writeln!(stdout, "{}", "-".repeat(50))
                .map_err(|error| format!("history search: terminal write failed: {}", error))?;

            let display_count = matches.len().min(12);
            for (i, (item, _)) in matches.iter().take(display_count).enumerate() {
                let result = if i == selected {
                    writeln!(stdout, "\x1b[46m\x1b[30m > {} \x1b[0m", item)
                } else {
                    writeln!(stdout, "   {}", item)
                };
                result.map_err(|error| {
                    format!("history search: terminal write failed: {}", error)
                })?;
            }

            stdout
                .flush()
                .map_err(|error| format!("history search: terminal flush failed: {}", error))?;

            let event = event::read()
                .map_err(|error| format!("history search: terminal input failed: {}", error))?;
            if let Event::Key(key) = event {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => break,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Enter, _) => {
                        if !matches.is_empty() && selected < matches.len() {
                            selection = Some(matches[selected].0.clone());
                        }
                        break;
                    }
                    (KeyCode::Up, _) => selected = selected.saturating_sub(1),
                    (KeyCode::Down, _) if selected + 1 < display_count => selected += 1,
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
        Ok(selection)
    }
}