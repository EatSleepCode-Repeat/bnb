use crate::safety::TrashEntry;
use std::fs;
use std::io;

pub fn run() -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Home directory not found")
    })?;
    let manifest_path = home.join(".bnb/trash/manifest.json");

    if !manifest_path.exists() {
        println!("Nothing to undo.");
        return Ok(());
    }

    let data = fs::read_to_string(&manifest_path)?;
    let mut entries: Vec<TrashEntry> = serde_json::from_str(&data).unwrap_or_default();

    if let Some(last) = entries.pop() {
        if last.trashed_path.exists() {
            if let Some(parent) = last.original_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&last.trashed_path, &last.original_path)?;
            println!("\x1b[32mRestored:\x1b[0m {:?}", last.original_path);
            fs::write(manifest_path, serde_json::to_string_pretty(&entries)?)?;
        } else {
            println!("Error: Staged file no longer exists in trash.");
        }
    } else {
        println!("Trash manifest is empty.");
    }

    Ok(())
}