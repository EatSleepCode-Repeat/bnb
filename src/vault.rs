use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{stdout, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct VaultData {
    pub version: u32,
    pub secrets: HashMap<String, String>,
}

fn get_vault_file_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let bnb_dir = home.join(".bnb");
    let _ = fs::create_dir_all(&bnb_dir);
    Some(bnb_dir.join("vault.json"))
}

fn get_machine_key() -> Vec<u8> {
    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "user".into());
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/home".into());
    let salt = "bnb_vault_salt_v1_secure_key_derivation";
    let seed = format!("{}:{}:{}", user, home, salt);

    let mut hash = [0u8; 32];
    for (i, byte) in seed.bytes().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(byte).wrapping_mul(31);
    }
    hash.to_vec()
}

fn encrypt_val(data: &str) -> String {
    let key = get_machine_key();
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        let k = key[i % key.len()];
        let pos_mask = ((i * 31) & 0xFF) as u8;
        out.push(b ^ k ^ pos_mask);
    }
    hex_encode(&out)
}

fn decrypt_val(hex_str: &str) -> Option<String> {
    let bytes = hex_decode(hex_str)?;
    let key = get_machine_key();
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        let k = key[i % key.len()];
        let pos_mask = ((i * 31) & 0xFF) as u8;
        out.push(b ^ k ^ pos_mask);
    }
    String::from_utf8(out).ok()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

pub fn load_vault() -> VaultData {
    let path = match get_vault_file_path() {
        Some(p) => p,
        None => return VaultData::default(),
    };

    if !path.exists() {
        return VaultData {
            version: 1,
            secrets: HashMap::new(),
        };
    }

    match fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or(VaultData {
            version: 1,
            secrets: HashMap::new(),
        }),
        Err(_) => VaultData {
            version: 1,
            secrets: HashMap::new(),
        },
    }
}

pub fn save_vault(data: &VaultData) -> Result<(), String> {
    let path = get_vault_file_path().ok_or_else(|| "cannot locate home directory".to_string())?;
    let json_str =
        serde_json::to_string_pretty(data).map_err(|e| format!("json serialize error: {}", e))?;

    #[cfg(unix)]
    {
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        options.mode(0o600);
        let mut file = options
            .open(&path)
            .map_err(|e| format!("cannot open vault file: {}", e))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| format!("cannot write vault file: {}", e))?;
    }

    #[cfg(not(unix))]
    {
        fs::write(&path, json_str).map_err(|e| format!("cannot write vault file: {}", e))?;
    }

    Ok(())
}

pub fn set_secret(key: &str, value: &str) -> Result<(), String> {
    let mut vault = load_vault();
    let enc = encrypt_val(value);
    vault.secrets.insert(key.to_string(), enc);
    save_vault(&vault)
}

pub fn get_secret(key: &str) -> Option<String> {
    let vault = load_vault();
    let enc = vault.secrets.get(key)?;
    decrypt_val(enc)
}

pub fn delete_secret(key: &str) -> Result<bool, String> {
    let mut vault = load_vault();
    let removed = vault.secrets.remove(key).is_some();
    if removed {
        save_vault(&vault)?;
    }
    Ok(removed)
}

pub fn list_secret_keys() -> Vec<String> {
    let vault = load_vault();
    let mut keys: Vec<String> = vault.secrets.keys().cloned().collect();
    keys.sort();
    keys
}

pub fn run_vault_cli(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return print_vault_status();
    }

    let subcmd = args[0].as_str();
    match subcmd {
        "set" => {
            let key = args
                .get(1)
                .ok_or_else(|| "usage: vault set <KEY_NAME>".to_string())?;

            if !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err("bnb: vault key must contain only [a-zA-Z0-9_-]".to_string());
            }

            print!("\x1b[1;36m🔐 Enter secret value for '{}': \x1b[0m", key);
            stdout().flush().ok();

            let secret_val = read_masked_password()?;

            if secret_val.trim().is_empty() {
                return Err("bnb: secret value cannot be empty".to_string());
            }

            set_secret(key, secret_val.trim())?;
            println!(
                "\x1b[1;32m✓ Secret '{}' securely saved to bnb vault.\x1b[0m",
                key
            );
            Ok(())
        }
        "get" => {
            let key = args
                .get(1)
                .ok_or_else(|| "usage: vault get <KEY_NAME>".to_string())?;
            match get_secret(key) {
                Some(val) => {
                    println!("{}", val);
                    Ok(())
                }
                None => Err(format!("bnb: secret '{}' not found in vault", key)),
            }
        }
        "remove" | "delete" | "rm" => {
            let key = args
                .get(1)
                .ok_or_else(|| "usage: vault remove <KEY_NAME>".to_string())?;
            if delete_secret(key)? {
                println!("\x1b[1;32m✓ Secret '{}' removed from vault.\x1b[0m", key);
                Ok(())
            } else {
                Err(format!("bnb: secret '{}' not found in vault", key))
            }
        }
        "list" => print_vault_status(),
        _ => Err(format!(
            "bnb: unknown vault subcommand '{}'. Usage: vault <set|get|list|remove> [KEY]",
            subcmd
        )),
    }
}

fn print_vault_status() -> Result<(), String> {
    let keys = list_secret_keys();
    if keys.is_empty() {
        println!("\x1b[1;33m🔐 bnb Secret Vault is empty.\x1b[0m");
        println!("Add a secret with: \x1b[1;36mvault set <KEY_NAME>\x1b[0m");
        return Ok(());
    }

    println!(
        "\x1b[1;36m🔐 bnb Secret Vault ({} stored):\x1b[0m",
        keys.len()
    );
    for key in &keys {
        println!(
            "  • \x1b[1;33m{:<20}\x1b[0m \x1b[38;5;244m[••••••••]\x1b[0m",
            key
        );
    }
    println!(
        "\n\x1b[38;5;244mReference in commands using $vault:KEY_NAME or ${{vault:KEY_NAME}}\x1b[0m"
    );
    Ok(())
}

fn read_masked_password() -> Result<String, String> {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut password = String::new();

    loop {
        if let Ok(Event::Key(key)) = event::read() {
            match (key.code, key.modifiers) {
                (KeyCode::Enter, _) => {
                    break;
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('d'), KeyModifiers::CONTROL)
                | (KeyCode::Esc, _) => {
                    let _ = disable_raw_mode();
                    println!();
                    return Err("bnb: input cancelled".to_string());
                }
                (KeyCode::Backspace, _) => {
                    if !password.is_empty() {
                        password.pop();
                        print!("\x08 \x08");
                        let _ = stdout().flush();
                    }
                }
                (KeyCode::Char(c), _) => {
                    password.push(c);
                    print!("*");
                    let _ = stdout().flush();
                }
                _ => {}
            }
        }
    }

    let _ = disable_raw_mode();
    println!();
    Ok(password)
}
