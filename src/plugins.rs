use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/EatSleepCode-Repeat/bnb-plugins/master/registry.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginHeader,
    #[serde(default)]
    pub action: Vec<PluginAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHeader {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "General".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAction {
    pub label: String,
    pub command: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: u32,
    pub plugins: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub path: Option<String>,
    pub repo: Option<String>,
}

pub struct PluginManager {
    plugins_dir: PathBuf,
}

impl PluginManager {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not resolve home directory");
        let plugins_dir = home.join(".bnb").join("plugins");
        if !plugins_dir.exists() {
            let _ = fs::create_dir_all(&plugins_dir);
        }
        Self { plugins_dir }
    }

    /// Load all installed plugin manifests from ~/.bnb/plugins/*/plugin.toml
    pub fn load_installed_plugins(&self) -> Vec<PluginManifest> {
        let mut installed = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("plugin.toml");
                    if manifest_path.exists() {
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = toml::from_str::<PluginManifest>(&content) {
                                installed.push(manifest);
                            }
                        }
                    }
                }
            }
        }
        installed
    }

    /// Load all active plugin actions for Ctrl+K integration
    pub fn load_all_actions(&self) -> Vec<(String, PluginAction)> {
        let mut actions = Vec::new();
        for plugin in self.load_installed_plugins() {
            let p_name = plugin.plugin.name.clone();
            for act in plugin.action {
                actions.push((p_name.clone(), act));
            }
        }
        actions
    }

    /// Fetch central registry database from GitHub
    pub fn fetch_registry(&self) -> Result<Registry, String> {
        let output = Command::new("curl")
            .args(["-sSL", REGISTRY_URL])
            .output()
            .map_err(|e| format!("Failed to execute curl: {}", e))?;

        if !output.status.success() {
            return Err("Failed to fetch registry from GitHub".to_string());
        }

        let body = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<Registry>(&body)
            .map_err(|e| format!("Invalid registry format: {}", e))
    }

    /// Install a plugin by name or GitHub repo shorthand (user/repo)
    pub fn install(&self, target: &str) -> Result<String, String> {
        if target.contains('/') {
            let repo_url = format!("https://github.com/{}.git", target);
            let folder_name = target.split('/').nth(1).unwrap_or(target);
            let dest = self.plugins_dir.join(folder_name);

            if dest.exists() {
                return Err(format!("Plugin '{}' is already installed.", folder_name));
            }

            println!("🧩 Cloning third-party plugin from {}...", repo_url);
            let status = Command::new("git")
                .args(["clone", &repo_url, dest.to_str().unwrap()])
                .status()
                .map_err(|e| format!("Git clone failed: {}", e))?;

            if status.success() {
                Ok(format!("Successfully installed '{}'", folder_name))
            } else {
                Err(format!("Failed to clone plugin repository '{}'", target))
            }
        } else {
            let registry = self.fetch_registry()?;
            let entry = registry
                .plugins
                .iter()
                .find(|p| p.name == target)
                .ok_or_else(|| format!("Plugin '{}' not found in registry.", target))?;

            let dest = self.plugins_dir.join(&entry.name);
            if dest.exists() {
                return Err(format!("Plugin '{}' is already installed.", entry.name));
            }

            fs::create_dir_all(&dest).map_err(|e| format!("Failed to create directory: {}", e))?;

            if entry.plugin_type == "official" {
                let path = entry.path.as_deref().unwrap_or("");
                let file_url = format!(
                    "https://raw.githubusercontent.com/EatSleepCode-Repeat/bnb-plugins/main/{}/plugin.toml",
                    path
                );

                println!("🧩 Downloading official plugin '{}'...", entry.name);
                let output = Command::new("curl")
                    .args([
                        "-sSL",
                        &file_url,
                        "-o",
                        dest.join("plugin.toml").to_str().unwrap(),
                    ])
                    .output()
                    .map_err(|e| format!("Failed to download plugin file: {}", e))?;

                if output.status.success() {
                    Ok(format!(
                        "Successfully installed official plugin '{}'",
                        entry.name
                    ))
                } else {
                    let _ = fs::remove_dir_all(&dest);
                    Err(format!(
                        "Failed to download plugin manifest for '{}'",
                        entry.name
                    ))
                }
            } else if let Some(repo) = &entry.repo {
                let repo_url = format!("https://github.com/{}.git", repo);
                println!("🧩 Cloning plugin repository from {}...", repo_url);
                let status = Command::new("git")
                    .args(["clone", &repo_url, dest.to_str().unwrap()])
                    .status()
                    .map_err(|e| format!("Git clone failed: {}", e))?;

                if status.success() {
                    Ok(format!("Successfully installed plugin '{}'", entry.name))
                } else {
                    let _ = fs::remove_dir_all(&dest);
                    Err(format!("Failed to clone plugin repo '{}'", repo))
                }
            } else {
                Err("Invalid registry entry configuration.".to_string())
            }
        }
    }

    /// Remove an installed plugin
    pub fn remove(&self, name: &str) -> Result<String, String> {
        let dest = self.plugins_dir.join(name);
        if !dest.exists() {
            return Err(format!("Plugin '{}' is not installed.", name));
        }
        fs::remove_dir_all(&dest).map_err(|e| format!("Failed to remove plugin: {}", e))?;
        Ok(format!("✓ Removed plugin '{}'", name))
    }

    /// Print all installed plugins and registered actions
    pub fn list(&self) {
        let plugins = self.load_installed_plugins();
        if plugins.is_empty() {
            println!("🧩 No plugins installed.");
            println!("  Install one with: bnb plugin install <name>");
            println!("  Or browse store:  bnb plugin store");
            return;
        }

        println!("🧩 Installed bnb-shell Plugins ({}):", plugins.len());
        for p in plugins {
            println!(
                "  • {} (v{}) - {}",
                p.plugin.name, p.plugin.version, p.plugin.description
            );
            for act in p.action {
                println!("      └─ Action: {} -> '{}'", act.label, act.command);
            }
        }
    }
}
