use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct TrashEntry {
    pub id: String,
    pub original_path: PathBuf,
    pub trash_path: PathBuf,
    pub deleted_at: u64,
    pub is_dir: bool,
    pub size_bytes: u64,
}

pub fn get_trash_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".bnb").join("trash")
}

fn get_manifest_path() -> PathBuf {
    get_trash_dir().join("manifest.txt")
}

pub fn move_to_trash(target: &Path) -> Result<TrashEntry, String> {
    let abs_target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    let trash_dir = get_trash_dir();
    fs::create_dir_all(&trash_dir)
        .map_err(|e| format!("trash: failed to create trash dir: {}", e))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp_sec = now.as_secs();
    let timestamp_nano = now.as_nanos();

    let file_name = abs_target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "deleted_item".to_string());

    let trash_id = format!("{}_{}", timestamp_nano, file_name);
    let trash_file_path = trash_dir.join(&trash_id);

    let metadata = fs::metadata(&abs_target)
        .map_err(|e| format!("trash: cannot read metadata for {}: {}", target.display(), e))?;

    let is_dir = metadata.is_dir();
    let size_bytes = if is_dir {
        get_dir_size(&abs_target).unwrap_or(0)
    } else {
        metadata.len()
    };

    fs::rename(&abs_target, &trash_file_path)
        .or_else(|_| copy_and_remove(&abs_target, &trash_file_path))
        .map_err(|e| format!("trash: failed to move {} to trash: {}", target.display(), e))?;

    let entry = TrashEntry {
        id: trash_id,
        original_path: abs_target,
        trash_path: trash_file_path,
        deleted_at: timestamp_sec,
        is_dir,
        size_bytes,
    };

    append_manifest(&entry)?;
    Ok(entry)
}

fn get_dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += get_dir_size(&p)?;
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

fn copy_and_remove(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        copy_dir_recursive(from, to)?;
        fs::remove_dir_all(from)?;
    } else {
        fs::copy(from, to)?;
        fs::remove_file(from)?;
    }
    Ok(())
}

fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target_path = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &target_path)?;
        } else {
            fs::copy(entry.path(), target_path)?;
        }
    }
    Ok(())
}

fn append_manifest(entry: &TrashEntry) -> Result<(), String> {
    let mut entries = list_trash_entries();
    entries.retain(|e| e.id != entry.id);
    entries.push(entry.clone());
    save_all_entries(&entries)
}

pub fn list_trash_entries() -> Vec<TrashEntry> {
    let manifest_path = get_manifest_path();
    if !manifest_path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut entries = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 6 {
            let entry = TrashEntry {
                id: parts[0].to_string(),
                original_path: PathBuf::from(parts[1]),
                trash_path: PathBuf::from(parts[2]),
                deleted_at: parts[3].parse().unwrap_or(0),
                is_dir: parts[4].parse().unwrap_or(false),
                size_bytes: parts[5].parse().unwrap_or(0),
            };
            if entry.trash_path.exists() {
                entries.push(entry);
            }
        }
    }

    entries.sort_by_key(|e| std::cmp::Reverse(e.deleted_at));
    entries
}

fn save_all_entries(entries: &[TrashEntry]) -> Result<(), String> {
    let manifest_path = get_manifest_path();
    let mut content = String::new();
    for entry in entries {
        content.push_str(&format!(
            "{}|{}|{}|{}|{}|{}\n",
            entry.id,
            entry.original_path.display(),
            entry.trash_path.display(),
            entry.deleted_at,
            entry.is_dir,
            entry.size_bytes
        ));
    }
    fs::write(manifest_path, content)
        .map_err(|e| format!("trash: cannot update manifest: {}", e))
}

pub fn restore_entry(entry: &TrashEntry) -> Result<(), String> {
    if let Some(parent) = entry.original_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if entry.original_path.exists() {
        return Err(format!(
            "cannot restore: target path '{}' already exists",
            entry.original_path.display()
        ));
    }

    fs::rename(&entry.trash_path, &entry.original_path)
        .or_else(|_| copy_and_remove(&entry.trash_path, &entry.original_path))
        .map_err(|e| format!("cannot restore file: {}", e))?;

    let mut entries = list_trash_entries();
    entries.retain(|e| e.id != entry.id);
    save_all_entries(&entries)?;

    Ok(())
}

pub fn purge_entry(entry: &TrashEntry) -> Result<(), String> {
    if entry.trash_path.is_dir() {
        let _ = fs::remove_dir_all(&entry.trash_path);
    } else {
        let _ = fs::remove_file(&entry.trash_path);
    }

    let mut entries = list_trash_entries();
    entries.retain(|e| e.id != entry.id);
    save_all_entries(&entries)?;

    Ok(())
}

pub fn clear_trash() -> Result<usize, String> {
    let entries = list_trash_entries();
    let count = entries.len();
    for entry in &entries {
        let _ = purge_entry(entry);
    }
    let manifest_path = get_manifest_path();
    let _ = fs::remove_file(manifest_path);
    Ok(count)
}