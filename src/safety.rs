pub mod trash;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct TrashEntry {
    pub original_path: PathBuf,
    pub trashed_path: PathBuf,
    pub timestamp: u64,
}

pub struct SafetyEngine;

impl SafetyEngine {
    /// Intercepts destructive commands before execution.
    /// Returns `true` if safe to proceed, `false` if cancelled by user.
    pub fn intercept(args: &[String]) -> bool {
        if args.is_empty() {
            return true;
        }

        let cmd = &args[0];
        let has_recursive = args.iter().any(|a| a == "-r" || a == "-rf" || a == "-fr" || a == "-R");

        if cmd == "rm" && has_recursive {
            println!("\x1b[33m⚠️  Guardrail: Recursive deletion requested for: {:?}\x1b[0m", &args[1..]);
            print!("Proceed with deletion? [y/N]: ");
            io::stdout().flush().ok();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap_or(0);
            return input.trim().eq_ignore_ascii_case("y");
        }

        true
    }

    /// Moves files to staging ~/.bnb/trash for soft-deletion support
    pub fn safe_remove(targets: &[String]) -> io::Result<()> {
        let home = dirs::home_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Home directory not found")
        })?;
        let trash_dir = home.join(".bnb/trash");
        fs::create_dir_all(&trash_dir)?;

        let mut entries = Vec::new();

        for target in targets {
            if target.starts_with('-') {
                continue;
            }
            let path = Path::new(target);
            if !path.exists() {
                continue;
            }

            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            let unique_name = format!("{}_{}", chrono::Utc::now().timestamp_millis(), file_name);
            let trashed_path = trash_dir.join(&unique_name);

            fs::rename(path, &trashed_path)?;

            entries.push(TrashEntry {
                original_path: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
                trashed_path,
                timestamp: chrono::Utc::now().timestamp() as u64,
            });
        }

        if !entries.is_empty() {
            Self::record_manifest(&trash_dir, entries)?;
        }

        Ok(())
    }

    fn record_manifest(trash_dir: &Path, new_entries: Vec<TrashEntry>) -> io::Result<()> {
        let manifest_path = trash_dir.join("manifest.json");
        let mut existing: Vec<TrashEntry> = if manifest_path.exists() {
            let data = fs::read_to_string(&manifest_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        existing.extend(new_entries);
        fs::write(manifest_path, serde_json::to_string_pretty(&existing)?)?;
        Ok(())
    }
}