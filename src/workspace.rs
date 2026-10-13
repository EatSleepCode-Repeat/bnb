use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorkspaceAction {
    pub display_label: String,
    pub command_to_run: String,
}

fn parse_makefile_targets(path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut targets = Vec::new();
    for line in content.lines() {
        if line.starts_with('\t') || line.starts_with(' ') {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((target_part, _)) = trimmed.split_once(':') {
            if target_part.contains('=')
                || target_part.starts_with('.')
                || target_part.contains('%')
            {
                continue;
            }
            for target in target_part.split_whitespace() {
                if target
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    && !targets.contains(&target.to_string())
                {
                    targets.push(target.to_string());
                }
            }
        }
    }
    targets
}

fn parse_taskfile_targets(path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut targets = Vec::new();
    let mut in_tasks = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if line.starts_with("tasks:") {
            in_tasks = true;
            continue;
        }

        if in_tasks
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && line.contains(':')
        {
            break;
        }

        if in_tasks {
            let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
            if leading_spaces == 2 && line.ends_with(':') {
                let task_name = trimmed.trim_end_matches(':').trim();
                if !task_name.is_empty()
                    && task_name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    && !targets.contains(&task_name.to_string())
                {
                    targets.push(task_name.to_string());
                }
            }
        }
    }
    targets
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

    // 6. Makefile Targets
    let makefile_names = ["Makefile", "makefile", "GNUmakefile"];
    for name in makefile_names {
        let path = dir.join(name);
        if path.is_file() {
            for target in parse_makefile_targets(&path) {
                actions.push(WorkspaceAction {
                    display_label: format!("⚡ [Workspace] 🛠️ make {}", target),
                    command_to_run: format!("make {}", target),
                });
            }
            break;
        }
    }

    // 7. Taskfile Targets
    let taskfile_names = [
        "Taskfile.yml",
        "Taskfile.yaml",
        "taskfile.yml",
        "taskfile.yaml",
    ];
    for name in taskfile_names {
        let path = dir.join(name);
        if path.is_file() {
            for target in parse_taskfile_targets(&path) {
                actions.push(WorkspaceAction {
                    display_label: format!("⚡ [Workspace] ⚡ task {}", target),
                    command_to_run: format!("task {}", target),
                });
            }
            break;
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
    if pwd.join("Makefile").exists()
        || pwd.join("makefile").exists()
        || pwd.join("GNUmakefile").exists()
    {
        badges.push("🛠️ Make");
    }
    if pwd.join("Taskfile.yml").exists()
        || pwd.join("Taskfile.yaml").exists()
        || pwd.join("taskfile.yml").exists()
        || pwd.join("taskfile.yaml").exists()
    {
        badges.push("⚡ Task");
    }

    if badges.is_empty() {
        String::new()
    } else {
        format!("\x1b[1;35m[{}]\x1b[0m ", badges.join(" "))
    }
}