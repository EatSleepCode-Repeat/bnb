use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::io::{stdout, Write};

pub struct FuzzyFinder;

impl FuzzyFinder {
    pub fn select(title: &str, items: &[String]) -> Option<String> {
        if items.is_empty() {
            return None;
        }

        enable_raw_mode().ok()?;
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen).ok()?;

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

            print!("\x1b[2J\x1b[1;1H");
            println!("\x1b[1;34m🔍 {} (Esc to cancel, Enter to pick)\x1b[0m", title);
            println!("Query: \x1b[33m{}\x1b[0m█", query);
            println!("{}", "-".repeat(50));

            let display_count = matches.len().min(12);
            for (i, (item, _)) in matches.iter().take(display_count).enumerate() {
                if i == selected {
                    println!("\x1b[46m\x1b[30m > {} \x1b[0m", item);
                } else {
                    println!("   {}", item);
                }
            }

            stdout.flush().ok();

            if let Ok(Event::Key(key)) = event::read() {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => break,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Enter, _) => {
                        if !matches.is_empty() && selected < matches.len() {
                            selection = Some(matches[selected].0.clone());
                        }
                        break;
                    }
                    (KeyCode::Up, _) => {
                        selected = selected.saturating_sub(1);
                    }
                    (KeyCode::Down, _) => {
                        if selected + 1 < display_count {
                            selected += 1;
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        query.pop();
                        selected = 0;
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        query.push(c);
                        selected = 0;
                    }
                    _ => {}
                }
            }
        }

        stdout.execute(LeaveAlternateScreen).ok()?;
        selection
    }
}