pub fn run(args: &[String]) -> Result<(), String> {
    // If the user runs `undo --last`, just restore the most recent file instantly
    if args.first().map(|s| s.as_str()) == Some("--last") {
        let entries = crate::safety::trash::list_trash_entries();
        if let Some(latest) = entries.first() {
            crate::safety::trash::restore_entry(latest)?;
            println!(
                "\x1b[1;32mRestored {}\x1b[0m",
                latest.original_path.display()
            );
            Ok(())
        } else {
            Err("trash: vault is empty".to_string())
        }
    } else {
        // Otherwise, open the interactive TUI
        crate::tui::trash_vault::run_trash_vault()
    }
}