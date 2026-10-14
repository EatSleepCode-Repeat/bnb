use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn get_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn check_tool(bin_name: &str, arg: &str) -> Option<String> {
    Command::new(bin_name)
        .arg(arg)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().and_then(|s| {
                    s.lines().next().map(|l| l.trim().to_string())
                })
            } else {
                None
            }
        })
}

pub fn run(_args: &[String], writer: &mut dyn Write) -> Result<(), String> {
    let mut warnings = 0;
    let mut checks_run = 0;

    writeln!(writer, "\x1b[1;36m🩺 bnb-shell Doctor Diagnostic Report\x1b[0m")
        .map_err(|e| e.to_string())?;
    writeln!(
        writer,
        "\x1b[38;5;240m─────────────────────────────────────────────────────────────\x1b[0m"
    )
    .map_err(|e| e.to_string())?;

    // 1. Config & Vault Storage
    writeln!(writer, "\x1b[1;37m📁 Configuration & Storage\x1b[0m").map_err(|e| e.to_string())?;

    if let Some(home) = get_home() {
        let bnb_dir = home.join(".bnb");
        checks_run += 1;
        if bnb_dir.exists() {
            writeln!(writer, "  \x1b[1;32m[✓]\x1b[0m State directory: {}", bnb_dir.display())
                .map_err(|e| e.to_string())?;
        } else {
            writeln!(writer, "  \x1b[1;33m[!]\x1b[0m State directory ~/.bnb not found (will be created automatically)")
                .map_err(|e| e.to_string())?;
            warnings += 1;
        }

        // Vault file & permissions check
        let vault_file = bnb_dir.join("vault.json");
        checks_run += 1;
        if vault_file.exists() {
            #[cfg(unix)]
            {
                if let Ok(meta) = fs::metadata(&vault_file) {
                    let mode = meta.permissions().mode() & 0o777;
                    if mode == 0o600 {
                        writeln!(
                            writer,
                            "  \x1b[1;32m[✓]\x1b[0m Secret vault: {} (secure mode 0600)",
                            vault_file.display()
                        )
                        .map_err(|e| e.to_string())?;
                    } else {
                        writeln!(
                            writer,
                            "  \x1b[1;33m[!]\x1b[0m Secret vault: {} has permissions {:o} (recommend 0600: `chmod 600 {}`)",
                            vault_file.display(),
                            mode,
                            vault_file.display()
                        )
                        .map_err(|e| e.to_string())?;
                        warnings += 1;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                writeln!(writer, "  \x1b[1;32m[✓]\x1b[0m Secret vault present: {}", vault_file.display())
                    .map_err(|e| e.to_string())?;
            }
        } else {
            writeln!(writer, "  \x1b[38;5;244m[-] Secret vault not yet initialized (run `vault set` to start)\x1b[0m")
                .map_err(|e| e.to_string())?;
        }

        // Plugin directory
        let plugin_dir = bnb_dir.join("plugins");
        checks_run += 1;
        if plugin_dir.exists() {
            let count = fs::read_dir(&plugin_dir)
                .map(|entries| entries.flatten().filter(|e| e.path().is_dir()).count())
                .unwrap_or(0);
            writeln!(
                writer,
                "  \x1b[1;32m[✓]\x1b[0m Plugin directory: {} ({} installed)",
                plugin_dir.display(),
                count
            )
            .map_err(|e| e.to_string())?;
        } else {
            writeln!(writer, "  \x1b[38;5;244m[-] Plugin directory not yet created\x1b[0m")
                .map_err(|e| e.to_string())?;
        }
    }

    writeln!(writer).map_err(|e| e.to_string())?;

    // 2. Developer Toolchain
    writeln!(writer, "\x1b[1;37m🛠️ Developer Toolchain\x1b[0m").map_err(|e| e.to_string())?;
    let tools = [
        ("git", "--version", true),
        ("cargo", "--version", false),
        ("rustc", "--version", false),
        ("docker", "--version", false),
        ("make", "--version", false),
        ("node", "--version", false),
        ("python3", "--version", false),
    ];

    for (name, flag, required) in tools {
        checks_run += 1;
        match check_tool(name, flag) {
            Some(ver) => {
                writeln!(writer, "  \x1b[1;32m[✓]\x1b[0m {:<8} \x1b[38;5;244m{}\x1b[0m", name, ver)
                    .map_err(|e| e.to_string())?;
            }
            None => {
                if required {
                    writeln!(writer, "  \x1b[1;31m[✗]\x1b[0m {:<8} (missing - core functionality degraded)", name)
                        .map_err(|e| e.to_string())?;
                    warnings += 1;
                } else {
                    writeln!(writer, "  \x1b[38;5;244m[-] {:<8} (not installed)\x1b[0m", name)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    writeln!(writer).map_err(|e| e.to_string())?;

    // 3. Terminal & Display Capabilities
    writeln!(writer, "\x1b[1;37m🖥️ Terminal & Environment\x1b[0m").map_err(|e| e.to_string())?;
    checks_run += 1;
    let term = env::var("TERM").unwrap_or_else(|_| "unknown".to_string());
    let colorterm = env::var("COLORTERM").unwrap_or_default();
    let has_truecolor = colorterm == "truecolor" || colorterm == "24bit";

    if has_truecolor {
        writeln!(writer, "  \x1b[1;32m[✓]\x1b[0m TrueColor 24-bit color support detected ($COLORTERM={})", colorterm)
            .map_err(|e| e.to_string())?;
    } else if term.contains("256color") {
        writeln!(writer, "  \x1b[1;32m[✓]\x1b[0m 256-color support ($TERM={})", term)
            .map_err(|e| e.to_string())?;
    } else {
        writeln!(writer, "  \x1b[1;33m[!]\x1b[0m Basic terminal detected ($TERM={}). Colors may be limited.", term)
            .map_err(|e| e.to_string())?;
        warnings += 1;
    }

    writeln!(writer).map_err(|e| e.to_string())?;

    // 4. PATH Hygiene
    writeln!(writer, "\x1b[1;37m🔍 $PATH Hygiene\x1b[0m").map_err(|e| e.to_string())?;
    if let Ok(path_var) = env::var("PATH") {
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();
        let mut nonexistent = Vec::new();

        for dir in env::split_paths(&path_var) {
            checks_run += 1;
            if !seen.insert(dir.clone()) {
                duplicates.push(dir.display().to_string());
            }
            if !dir.exists() {
                nonexistent.push(dir.display().to_string());
            }
        }

        if duplicates.is_empty() && nonexistent.is_empty() {
            writeln!(writer, "  \x1b[1;32m[✓]\x1b[0m All {} PATH entries are unique and exist on disk.", seen.len())
                .map_err(|e| e.to_string())?;
        } else {
            if !duplicates.is_empty() {
                warnings += 1;
                writeln!(writer, "  \x1b[1;33m[!]\x1b[0m Found {} duplicate PATH entries:", duplicates.len())
                    .map_err(|e| e.to_string())?;
                for dup in duplicates {
                    writeln!(writer, "      └─ {}", dup).map_err(|e| e.to_string())?;
                }
            }
            if !nonexistent.is_empty() {
                warnings += 1;
                writeln!(writer, "  \x1b[1;33m[!]\x1b[0m Found {} non-existent PATH entries:", nonexistent.len())
                    .map_err(|e| e.to_string())?;
                for ne in nonexistent {
                    writeln!(writer, "      └─ {}", ne).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    writeln!(
        writer,
        "\x1b[38;5;240m─────────────────────────────────────────────────────────────\x1b[0m"
    )
    .map_err(|e| e.to_string())?;

    // Summary
    if warnings == 0 {
        writeln!(writer, "\x1b[1;32m✓ All {} checks passed! No issues detected.\x1b[0m", checks_run)
            .map_err(|e| e.to_string())?;
    } else {
        writeln!(
            writer,
            "\x1b[1;33m⚠️ Diagnostics completed: {} checks run, {} warning(s) found.\x1b[0m",
            checks_run, warnings
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}