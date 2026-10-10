# bnb

`bnb` is a Rust command shell for users who want a polished, zsh-inspired interactive workflow with a small, explicit shell-language surface. It provides a Git-aware prompt, command-line editing, history search and suggestions, common command composition, directory navigation, and configurable aliases and environment variables.

---

## Features

- **Interactive TUI Overlays** — Sleek modal popups with rounded borders, contextual icons, live fuzzy character match highlighting, and sliding window scrolling:
  - **Universal Command Palette (`Ctrl+K`)** — Single interactive launcher to search, discover, and trigger every TUI tool, shell action, or configuration helper.
  - **Visual Trash Vault (`Ctrl+U`)** — Interactive modal displaying soft-deleted files, sizes, and timestamps. Select individual or multiple items (`Tab`/`Space`) to restore or permanently purge.
  - **History Search (`Ctrl+R`)** — Fuzzy search and select past commands from shell history.
  - **Fuzzy File Finder (`Ctrl+F`)** — Search project files recursively (skipping `.git`, `node_modules`, `target`, etc.) and insert paths directly into your active prompt.
  - **Git Branch Switcher (`Ctrl+B`)** — Inspect local (`🌿`) and remote (`🌐`) branches and switch branches effortlessly.
  - **Multi-Select Git Stager (`Ctrl+G`)** — Visual status TUI allowing bulk file staging (`git add`) and unstaging (`git restore --staged`) using checkbox selection.
  - **Global Project Switcher (`Ctrl+O`)** — Scans your system for local Git repositories and enables instant directory hopping.
  - **Directory Jumper (`Ctrl+Z`)** — Jump directly to your most frequent directories (`📁`) powered by `z` frecency scores.
  - **Prompt Theme Wizard (`Ctrl+T`)** — Interactive setup wizard with live prompt previews to customize layout, colors, and Nerd Font icons.
- **Smart Typo Auto-Correction** — Detects mistyped commands using Levenshtein distance on command failure (`127`) and offers instant one-key (`Enter`) execution of the corrected command.
- **Prompt & Git Integration** — Dynamic Powerline or Lean rendering of OS, directory path, Git branch state (dirty, untracked, ahead/behind counters), and execution time for long-running commands.
- **Safety Guardrail (`rm` & `undo`)** — The `rm` builtin intercepts deletions and safely moves files to `~/.bnb/trash/` with nanosecond-unique timestamps. The `undo` builtin restores them visually or instantly via `--last`.
- **Real-Time Syntax Highlighting** — Visual status for valid commands (green), invalid commands (red), operators (yellow), strings (cyan), and environment variables (magenta).
- **Inline Autosuggestions** — Fish-style history completion as you type, accepted using the right arrow key (`→`).
- **Command Completion** — Completes builtins, aliases, commands from `PATH`, and file paths.
- **Command & Parameter Substitution** — Full support for nested command execution (`$(cmd)` and `` `cmd` ``), environment parameters (`$NAME`, `${NAME}`), wildcards, brace expansions, exit status (`$?`), and process ID (`$$`).
- **Frecency Directory Navigation (`z`)** — Jumps to frequently used directories based on rank and recency, including an interactive selection mode (`z -i`).
- **Pipelines and Redirection** — Supports conditional chaining (`&&`, `||`), sequencing (`;` and newlines), background launch (`&`), pipes (`|`), and common file redirections (`>`, `>>`, `<`, `2>`, `2>>`, `&>`).
- **Expansion Support** — Supports tilde, unquoted simple wildcards (`*`, `?`), brace ranges (`{1..5}`, `{a..e}`), comma expansion (`{foo,bar}`), environment parameters (`$NAME`, `${NAME}`), common parameter defaults/length/prefix-suffix removal, exit status (`$?`), and process ID (`$$`). Quoted wildcard characters remain literal.
- **Shell Assignments** — Supports persistent `NAME=value` assignments and temporary command-prefix assignments such as `EDITOR=vim command`.
- **Config & Sourcing** — Loads only `~/.bnbrc` on startup and accepts simple `export KEY=VALUE` and `alias NAME=VALUE` statements. It does not read `~/.zshrc` or `~/.bashrc`. `source` and `.` can explicitly load a file containing the same supported subset; they do not execute arbitrary shell scripts.
- **Explicit Safety Boundary** — Unsupported config lines are reported with file and line numbers, keeping the shell intentionally small and predictable instead of silently accepting unsupported zsh syntax.

---

## Installation

### From Crates.io

```bash
cargo install bnb-shell
```

### From Source

```bash
git clone [https://github.com/EatSleepCode-Repeat/bnb.git](https://github.com/EatSleepCode-Repeat/bnb.git)
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
| :---------------- | :-------------------------------------------------- |
| `-c <command>`    | Execute a command string non-interactively and exit |
| `-h`, `--help`    | Print usage information and list available builtins |
| `-V`, `--version` | Display version information                         |

---

## Keybindings

| Shortcut          | Action                                          |
| :---------------- | :---------------------------------------------- |
| `Ctrl + K`        | Open Universal Command Palette                  |
| `Ctrl + U`        | Open Visual Trash Vault modal                   |
| `Ctrl + T`        | Open Prompt Theme Configuration Wizard          |
| `Ctrl + G`        | Open Multi-Select Git Status Stager             |
| `Ctrl + O`        | Open Global Project Switcher                    |
| `Ctrl + R`        | Open interactive TUI history search             |
| `Ctrl + F`        | Open interactive TUI file finder                |
| `Ctrl + B`        | Open interactive TUI Git branch switcher        |
| `Ctrl + Z`        | Open interactive TUI frecency directory jumper  |
| `Ctrl + A`        | Move to the beginning of the input line         |
| `Ctrl + E`        | Move to the end of the input line               |
| `Ctrl + L`        | Clear the terminal screen                       |
| `Right Arrow (→)` | Accept inline autosuggestion                    |
| `Tab`             | Complete a command or path                      |
| `Ctrl + C`        | Cancel active input buffer or interrupt process |
| `Ctrl + D`        | Send EOF / exit shell session                   |

---

## Builtin Commands

| Command                      | Description                                                                    |
| :--------------------------- | :----------------------------------------------------------------------------- |
| `rm <file ...>`              | Intercepted safety guardrail; soft-deletes items into `~/.bnb/trash/`          |
| `undo [--last]`              | Open Visual Trash Vault TUI or restore the most recently trashed file          |
| `prompt-config`              | Launch the interactive prompt theme configuration wizard                       |
| `cd [dir \| -]`              | Change working directory (supports `cd -` and auto `.env` loading)             |
| `mkcd <dir>`                 | Create directory hierarchy (`mkdir -p`) and switch into it                     |
| `z <query>`                  | Jump to highest frecency matching directory                                    |
| `z -i [query]`               | Interactively pick from top matching directories in a TUI popup                |
| `which <cmd>` / `type <cmd>` | Identify command source (builtin, alias, or file path)                         |
| `alias [key=val]`            | Set or display command aliases                                                 |
| `unalias <name \| -a>`       | Remove specific alias or clear all (`-a`)                                      |
| `export [key=val]`           | Set or print environment variables                                             |
| `unset <key ...>`            | Remove environment variables                                                   |
| `source <file>` / `. <file>` | Read and execute supported config statements from a file in the current shell  |
| `history [-c]`               | Print command history or clear it (`-c`)                                       |
| `echo [-n] [-e] <text>`      | Print text with option for trailing newline (`-n`) and escape sequences (`-e`) |
| `pwd`                        | Print current working directory                                                |
| `clear`                      | Clear terminal screen                                                          |
| `bnb-update`                 | Check crates.io and update binary in place                                     |
| `exit [code]`                | Terminate shell session with exit code                                         |

---

## Prompt & Theme Customization

`bnb` features a dynamic, interactive theme configuration wizard allowing you to customize your prompt aesthetic without touching config files.

Launch the wizard anytime using **`Ctrl+T`** or by running the **`prompt-config`** builtin.

Your configuration is automatically saved and loaded from `~/.bnb_prompt.conf`.

- **Styles**: Choose between `powerline` (segmented color blocks), `lean` (modern colored text), or `minimal` (clean, single-line).
- **Separators**: Select your preferred segment connections, including `angled` (``), `rounded` (``), `slanted` (``), or `none`.
- **Icons**: Toggle Nerd Font glyphs on or off for strict standard terminal compatibility.
- **Color Schemes**: Choose from 6 presets including Catppuccin Mocha, Dracula, Nord, Gruvbox Dark, Tokyo Night, and Classic.

## Configuration

Place supported aliases and exports in `~/.bnbrc`. An example template is available in [.bnbrc.example](./.bnbrc.example).

```bash
# Example ~/.bnbrc (supported syntax)
export EDITOR=vim
alias ll="ls -la"
alias g="git"
```

Reload configuration in an active session:

```bash
source ~/.bnbrc
```

`source` currently accepts the same config subset; it does not run arbitrary shell scripts.

## Compatibility boundary

bnb is zsh-inspired, not a zsh implementation. Quotes and escapes are checked, and common command composition and expansions are available. Startup reads only `.bnbrc`; `.zshrc` is left to zsh. If you explicitly `source` a file, unsupported statements are reported with file and line numbers.

The current shell does not implement shell functions, arrays, loops/conditionals, process substitution, advanced zsh glob qualifiers/options, or full interactive job control. Parameter default operands are literal strings, and prefix/suffix removal supports literal matches rather than full zsh pattern expressions. Script files are processed one line at a time. Background launch is not equivalent to zsh job management. These boundaries are intentional; do not rely on bnb to run arbitrary zsh or bash scripts.

Unknown external commands are resolved and executed directly by bnb. bnb does not reconstruct command text and pass it to a system zsh as a fallback.

---

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
