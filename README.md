# bnb

`bnb` is a fast, Rust command shell built for developers who want a polished, zsh-inspired interactive workflow with zero-latency TUI tools, contextual workspaces, and enhanced local security out of the box.

---

## Features

- **Adaptive Workspace Engine** — Automatically detects project manifests (`Cargo.toml`, `package.json`, `go.mod`, `docker-compose.yml`, `pyproject.toml`) upon entering a directory. Displays real-time stack badges in your prompt (e.g., `[🦀 Rust 🐳 Docker]`) and injects dynamic, stack-aware commands into your Command Palette.
- **Lightweight Plugin Engine (`bnb plugin`)** — Decoupled manifest model (`plugin.toml`) backed by a GitHub monorepo registry (`registry.json`). Install official or third-party plugins directly (`plugin install <name|user/repo>`), browse the registry marketplace (`plugin store`), and automatically expose community actions in `Ctrl+K` with zero shell startup overhead.
- **Zero-Leak Secret Vault (`bnb vault`)** — Encrypted local key-value secret store (`~/.bnb/vault.json` with strict `0600` permissions). Reference secrets directly in commands via `$vault:KEY_NAME` or `${vault:KEY_NAME}` to expand values in-memory at execution time, preventing secrets from leaking into `~/.bnb_history` or process argument lists (`ps aux`).
- **Multi-Line Shell Functions** — Full support for shell function definitions (`name() { ... }` and `function name { ... }`) with multi-line interactive block continuation (` >` prompt) and parameter expansion (`$1`, `$2`, `$@`, `$*`).
- **Interactive Safety Interceptor** — Styled TUI guardrail modal intercepting destructive operations (`rm -rf *`, `git push --force`, `dd of=/dev/sd*`, `mkfs`, `chmod -R /`). Prompts for user confirmation before execution.
- **Universal Command Palette (`Ctrl+K`)** — Single interactive launcher to search, discover, and trigger every TUI tool, workspace shortcut, plugin action, shell command, or configuration helper.
- **Visual Trash Vault (`Ctrl+U`)** — Interactive modal displaying soft-deleted files, sizes, and timestamps. Select individual or multiple items (`Tab`/`Space`) to restore or permanently purge.
- **Interactive Alias Manager (`alias-config`)** — Dedicated TUI wizard to add, edit, search, and remove persistent aliases stored in `~/.bnbrc`.
- **History Search (`Ctrl+R`)** — Fuzzy search and select past commands from shell history.
- **Fuzzy File Finder (`Ctrl+F`)** — Search project files recursively (skipping `.git`, `node_modules`, `target`, etc.) and insert paths directly into your active prompt.
- **Directory Overview (`Ctrl+K`)** — Browse the current directory in a two-pane view, preview folders and text files, change into a chosen folder, or insert a file path into your prompt.
- **Git Branch Switcher (`Ctrl+B`)** — Inspect local (`🌿`) and remote (`🌐`) branches and switch branches effortlessly.
- **Multi-Select Git Stager (`Ctrl+G`)** — Visual status TUI allowing bulk file staging (`git add`) and unstaging (`git restore --staged`) using checkbox selection.
- **Global Project Switcher (`Ctrl+O`)** — Scans your system for local Git repositories and enables instant directory hopping.
- **Directory Jumper (`Ctrl+Z`)** — Jump directly to your most frequent directories (`📁`) powered by `z` frecency scores.
- **Prompt Theme Wizard (`Ctrl+T`)** — Interactive setup wizard with live prompt previews to customize layout, colors, and Nerd Font icons.
- **Smart Typo Auto-Correction** — Detects mistyped commands using Levenshtein distance on command failure (`127`) and offers instant one-key (`Enter`) execution of the corrected command.
- **Prompt & Stack Integration** — Dynamic Powerline, Lean, or Minimal rendering of OS icon, directory path, Git branch state (dirty, untracked, ahead/behind counters), execution timer, and active stack badges.
- **Safety Guardrail (`rm` & `undo`)** — The `rm` builtin soft-deletes items into `~/.bnb/trash/` with nanosecond-unique timestamps. The `undo` builtin restores them visually or instantly via `--last`.
- **Real-Time Syntax Highlighting** — Visual status for valid commands (green), invalid commands (red), operators (yellow), strings (cyan), and environment variables (magenta).
- **Inline Autosuggestions** — Fish-style history completion as you type, accepted using the right arrow key (`→`).
- **Command & Parameter Substitution** — Support for nested command execution (`$(cmd)` and `` `cmd` ``), environment parameters (`$NAME`, `${NAME}`), vault variables (`$vault:KEY`), wildcards, brace expansions, exit status (`$?`), and process ID (`$$`).
- **Frecency Directory Navigation (`z`)** — Jumps to frequently used directories based on rank and recency, including an interactive selection mode (`z -i`).
- **Pipelines and Redirection** — Supports conditional chaining (`&&`, `||`), sequencing (`;` and newlines), background launch (`&`), pipes (`|`), and common file redirections (`>`, `>>`, `<`, `2>`, `2>>`, `&>`).
- **Config & Sourcing** — Loads `~/.bnbrc` on startup. Accepts simple `export KEY=VALUE`, `alias NAME=VALUE`, and function definitions. `source` and `.` explicitly load configuration files.

---

## Installation

### From Crates.io

```bash
cargo install bnb-shell
```

### From Source

```bash
git clone https://github.com/EatSleepCode-Repeat/bnb.git
cd bnb
cargo install --path .
```

To set `bnb` as your default system shell:

```bash
which bnb | sudo tee -a /etc/shells
chsh -s $(which bnb)
```

---

## CLI Options

| Flag              | Description                                         |
| ----------------- | --------------------------------------------------- |
| `-c <command>`    | Execute a command string non-interactively and exit |
| `-h`, `--help`    | Print usage information and list available builtins |
| `-V`, `--version` | Display version information                         |

---

## Keybindings

| Shortcut          | Action                                                      |
| ----------------- | ----------------------------------------------------------- |
| `Ctrl + K`        | Open Universal Command Palette (Workspace & Plugin actions) |
| `Ctrl + U`        | Open Visual Trash Vault modal                               |
| `Ctrl + T`        | Open Prompt Theme Configuration Wizard                      |
| `Ctrl + G`        | Open Multi-Select Git Status Stager                         |
| `Ctrl + O`        | Open Global Project Switcher                                |
| `Ctrl + R`        | Open interactive TUI history search                         |
| `Ctrl + F`        | Open interactive TUI file finder                            |
| `Ctrl + B`        | Open interactive TUI Git branch switcher                    |
| `Ctrl + Z`        | Open interactive TUI frecency directory jumper              |
| `Ctrl + A`        | Move to the beginning of the input line                     |
| `Ctrl + E`        | Move to the end of the input line                           |
| `Ctrl + L`        | Clear the terminal screen                                   |
| `Right Arrow (→)` | Accept inline autosuggestion                                |
| `Tab`             | Complete a command or path                                  |
| `Ctrl + C`        | Cancel active input buffer or interrupt process             |
| `Ctrl + D`        | Send EOF / exit shell session                               |

---

## Builtin Commands

| Command                              | Description                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------------ |
| `plugin <store/install/list/remove>` | Browse central registry marketplace and manage installed shell plugins         |
| `vault <set/get/list/remove>`        | Manage local encrypted secret vault (`$vault:KEY` expansion)                   |
| `functions`                          | List all currently defined multi-line shell functions                          |
| `unfunction <name>`                  | Remove a defined shell function from memory                                    |
| `alias-config`                       | Launch the interactive TUI Alias Manager                                       |
| `rm <file ...>`                      | Intercepted safety guardrail; soft-deletes items into `~/.bnb/trash/`          |
| `undo [--last]`                      | Open Visual Trash Vault TUI or restore the most recently trashed file          |
| `prompt-config`                      | Launch the interactive prompt theme configuration wizard                       |
| `cd [dir -]`                         | Change working directory (supports `cd -` and stack badge auto-detection)      |
| `mkcd <dir>`                         | Create directory hierarchy (`mkdir -p`) and switch into it                     |
| `z <query>`                          | Jump to highest frecency matching directory                                    |
| `z -i [query]`                       | Interactively pick from top matching directories in a TUI popup                |
| `which <cmd>` / `type <cmd>`         | Identify command source (builtin, function, alias, or binary path)             |
| `alias [key=val]`                    | Set or display command aliases                                                 |
| `unalias <name -a>`                  | Remove specific alias or clear all (`-a`)                                      |
| `export [key=val]`                   | Set or print environment variables                                             |
| `unset <key ...>`                    | Remove environment variables                                                   |
| `source <file>` / `. <file>`         | Read and execute supported config statements from a file in current shell      |
| `history [-c]`                       | Print command history or clear it (`-c`)                                       |
| `echo [-n] [-e] <text>`              | Print text with option for trailing newline (`-n`) and escape sequences (`-e`) |
| `pwd`                                | Print current working directory                                                |
| `clear`                              | Clear terminal screen                                                          |
| `cyberpunk`                          | Visual TUI celebration / fireworks animation                                   |
| `bnb-update`                         | Check crates.io and update binary in place                                     |
| `exit [code]`                        | Terminate shell session with exit code                                         |

---

## Usage Examples

### Using the Plugin System (`bnb plugin`)

```bash
# Browse central registry marketplace
plugin store

# Install an official plugin from the monorepo registry
plugin install docker

# Or install a third-party plugin directly from GitHub
plugin install username/bnb-custom-plugin

# List installed plugins & registered Ctrl+K shortcuts
plugin list

# Remove an installed plugin
plugin remove docker

```

### Using Secret Vault (`bnb vault`)

```bash
# Save a secret (input is masked)
vault set STRIPE_SECRET

# Reference in any shell command without history leakage
curl -H "Authorization: Bearer $vault:STRIPE_SECRET" [https://api.stripe.com/v1/charges](https://api.stripe.com/v1/charges)

# Or use braced syntax
echo "Connecting with ${vault:STRIPE_SECRET}"

```

### Writing Shell Functions

```bash
# Interactively define a function across multiple lines
mkcd() {
    mkdir -p "$1"
    cd "$1"
}

# Invoke function with positional arguments
mkcd my_new_project

```

---

## Prompt & Theme Customization

`bnb` features a dynamic, interactive theme configuration wizard allowing you to customize your prompt aesthetic without touching config files.

Launch the wizard anytime using **`Ctrl+T`** or by running the **`prompt-config`** builtin.

Your configuration is saved and loaded automatically from `~/.bnb_prompt.conf`.

- **Styles**: Choose between `powerline` (segmented color blocks), `lean` (modern colored text), or `minimal` (clean, single-line).
- **Separators**: Select your preferred segment connections, including `angled` (``), `rounded` (``), `slanted` (``), or `none`.
- **Icons**: Toggle Nerd Font glyphs on or off for strict standard terminal compatibility.
- **Color Schemes**: Choose from 6 presets including Catppuccin Mocha, Dracula, Nord, Gruvbox Dark, Tokyo Night, and Classic.

---

## Configuration

Place supported aliases, exports, and function definitions in `~/.bnbrc`.

```bash
# Example ~/.bnbrc
export EDITOR=vim
alias ll="ls -la"
alias g="git"

# Custom shell functions
gcommit() {
    git add .
    git commit -m "$1"
}

```

Reload configuration in an active session:

```bash
source ~/.bnbrc

```

---

## Compatibility Boundary

`bnb` is an interactive developer shell with explicit scripting capabilities. Startup reads `~/.bnbrc`; `.zshrc` is left to `zsh`. If you explicitly `source` a file, unsupported statements are reported with file and line numbers.

Multi-line functions, positional parameters (`$1`, `$@`), plugin manifests, vault expansions, alias management, and interactive block continuation are natively supported. Full control flow structures (`if`/`else` conditionals, `for`/`while` loops) and background job tables (`fg`/`bg`) are slated for the upcoming `v1.0` release.

---

## License

Distributed under the MIT License. See [LICENSE](https://www.google.com/search?q=LICENSE) for details.

```

```
