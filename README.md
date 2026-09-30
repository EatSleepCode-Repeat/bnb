# bnb

`bnb` is a Unix shell written in Rust, designed for high performance and zero configuration out of the box. It features Powerlevel10k-style prompt formatting, real-time syntax highlighting, history autosuggestions, interactive fuzzy searching, and soft-delete safety guardrails.

---

## Features

- **Prompt & Git Integration** — Two-line prompt rendering OS, directory path, Git branch state (dirty, untracked, ahead/behind counters), and execution time for long-running commands.
- **Interactive History Search (`Ctrl+R`)** — Built-in TUI overlay for searching and selecting past commands from shell history.
- **Safety Guardrails & Undo Engine** — Intercepts high-risk operations (such as `rm -rf`) with confirmation prompts, routes deleted files to `~/.bnb/trash/`, and provides a native `undo` builtin for restoration.
- **Real-Time Syntax Highlighting** — Visual status for valid commands (green), invalid commands (red), operators (yellow), strings (cyan), and environment variables (magenta).
- **Inline Autosuggestions** — Fish-style history completion as you type, accepted using the right arrow key (`→`).
- **Frecency Directory Navigation (`z`)** — Jumps to frequently used directories based on rank and recency, including an interactive selection mode (`z -i`).
- **Pipelines and Redirection** — Standard support for conditional chaining (`&&`, `||`), sequencing (`;`), background tasks (`&`), pipes (`|`), and file redirections (`>`, `>>`, `<`, `2>`, `&>`).
- **Expansion Support** — Wildcards (`*`, `?`), brace ranges (`{1..5}`, `{a..e}`), comma expansion (`{foo,bar}`), environment parameters (`${VAR}`), exit codes (`$?`), and process IDs (`$$`).
- **Config & Sourcing** — Loads `~/.bnbrc` on startup. Configuration can be reloaded live with `source ~/.bnbrc`.

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
| `Ctrl + R`        | Open interactive TUI history search             |
| `Right Arrow (→)` | Accept inline autosuggestion                    |
| `Ctrl + C`        | Cancel active input buffer or interrupt process |
| `Ctrl + D`        | Send EOF / exit shell session                   |

---

## Builtin Commands

| Command                      | Description                                                                    |
| :--------------------------- | :----------------------------------------------------------------------------- |
| `cd [dir \| -]`              | Change working directory (supports `cd -` for previous path)                   |
| `mkcd <dir>`                 | Create directory hierarchy (`mkdir -p`) and switch into it                     |
| `z <query>`                  | Jump to highest frecency matching directory                                    |
| `z -i [query]`               | Interactively pick from top matching directories                               |
| `undo`                       | Restore the last soft-deleted item from `~/.bnb/trash/`                        |
| `which <cmd>` / `type <cmd>` | Identify command source (builtin, alias, or file path)                         |
| `alias [key=val]`            | Set or display command aliases                                                 |
| `unalias <name \| -a>`       | Remove specific alias or clear all (`-a`)                                      |
| `export [key=val]`           | Set or print environment variables                                             |
| `unset <key ...>`            | Remove environment variables                                                   |
| `source <file>` / `. <file>` | Read and execute commands from a file in the current shell                     |
| `history [-c]`               | Print command history or clear it (`-c`)                                       |
| `echo [-n] [-e] <text>`      | Print text with option for trailing newline (`-n`) and escape sequences (`-e`) |
| `pwd`                        | Print current working directory                                                |
| `clear`                      | Clear terminal screen                                                          |
| `bnb-update`                 | Check crates.io and update binary in place                                     |
| `exit [code]`                | Terminate shell session with exit code                                         |

---

## Configuration

Place custom aliases, exports, and startup logic in `~/.bnbrc`. An example template is available in `.bnbrc.example`.

```bash
# Example ~/.bnbrc
export EDITOR=vim
alias ll="ls -la"
alias g="git"
```

Reload configuration in an active session:

```bash
source ~/.bnbrc
```

---

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
