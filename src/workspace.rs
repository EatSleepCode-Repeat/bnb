use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorkspaceAction {
    pub display_label: String,
    pub command_to_run: String,
}

pub fn detect_workspace_actions(dir: &Path) -> Vec<WorkspaceAction> {
    let mut actions = Vec::new();

    // 1. Rust Projects (Cargo.toml)
    if dir.join("Cargo.toml").exists() {
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🦀 Cargo Run (cargo run)".into(),
            command_to_run: "cargo run".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🦀 Cargo Test (cargo test)".into(),
            command_to_run: "cargo test".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🦀 Cargo Build (cargo build)".into(),
            command_to_run: "cargo build".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🦀 Cargo Clippy (cargo clippy)".into(),
            command_to_run: "cargo clippy -- -D warnings".into(),
        });
    }

    // 2. Node.js Projects (package.json - Dynamically parses scripts)
    let pkg_json = dir.join("package.json");
    if pkg_json.exists() {
        if let Ok(content) = fs::read_to_string(&pkg_json) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
                    for (script_name, _) in scripts {
                        actions.push(WorkspaceAction {
                            display_label: format!("⚡ [Workspace] 📦 npm run {}", script_name),
                            command_to_run: format!("npm run {}", script_name),
                        });
                    }
                }
            }
        }
        if actions.iter().all(|a| !a.command_to_run.starts_with("npm")) {
            actions.push(WorkspaceAction {
                display_label: "⚡ [Workspace] 📦 npm install".into(),
                command_to_run: "npm install".into(),
            });
        }
    }

    // 3. Go Projects (go.mod)
    if dir.join("go.mod").exists() {
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐹 Go Run (go run .)".into(),
            command_to_run: "go run .".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐹 Go Test (go test ./...)".into(),
            command_to_run: "go test ./...".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐹 Go Build (go build)".into(),
            command_to_run: "go build".into(),
        });
    }

    // 4. Docker Compose
    if dir.join("docker-compose.yml").exists() || dir.join("docker-compose.yaml").exists() {
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐳 Docker Compose Up (docker compose up -d)".into(),
            command_to_run: "docker compose up -d".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐳 Docker Compose Down (docker compose down)".into(),
            command_to_run: "docker compose down".into(),
        });
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐳 Docker Compose Logs (docker compose logs -f)".into(),
            command_to_run: "docker compose logs -f".into(),
        });
    }

    // 5. Python Projects
    if dir.join("pyproject.toml").exists()
        || dir.join("requirements.txt").exists()
        || dir.join("Pipfile").exists()
    {
        actions.push(WorkspaceAction {
            display_label: "⚡ [Workspace] 🐍 Pytest (pytest)".into(),
            command_to_run: "pytest".into(),
        });
        if dir.join("requirements.txt").exists() {
            actions.push(WorkspaceAction {
                display_label: "⚡ [Workspace] 🐍 Pip Install (pip install -r requirements.txt)"
                    .into(),
                command_to_run: "pip install -r requirements.txt".into(),
            });
        }
    }

    actions
}

pub fn get_workspace_badge() -> String {
    let Ok(pwd) = env::current_dir() else {
        return String::new();
    };

    let mut badges = Vec::new();

    if pwd.join("Cargo.toml").exists() {
        badges.push("🦀 Rust");
    }
    if pwd.join("package.json").exists() {
        badges.push("📦 Node");
    }
    if pwd.join("go.mod").exists() {
        badges.push("🐹 Go");
    }
    if pwd.join("docker-compose.yml").exists() || pwd.join("docker-compose.yaml").exists() {
        badges.push("🐳 Docker");
    }
    if pwd.join("pyproject.toml").exists()
        || pwd.join("requirements.txt").exists()
        || pwd.join("Pipfile").exists()
    {
        badges.push("🐍 Python");
    }

    if badges.is_empty() {
        String::new()
    } else {
        format!("\x1b[1;35m[{}]\x1b[0m ", badges.join(" "))
    }
}
