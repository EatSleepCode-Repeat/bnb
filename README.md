# ⚡ bnb

A fast, cross-platform terminal shell built in Rust with Powerlevel10k aesthetics, live syntax highlighting, history autosuggestions, command chaining, and frecency-based directory jumping. Designed to deliver the power of a fully loaded Zsh setup with instant `<5ms` startup and zero configuration.

---

## ✨ Features

- **🎨 Powerlevel10k Prompt** — Two-line dynamic prompt featuring OS indicator (``, ``, ``), path abbreviation (`~`), Git branch with status tracking (amber pill with `*` when dirty, `?` for untracked files, `⇡`/`⇣` for ahead/behind commits), and right-aligned execution duration (`took 1.4s ⌛`).
- **⚡ Real-Time Syntax Highlighting** — Commands and builtins highlight in green as you type, unresolved commands in red, shell operators (`&&`, `||`, `;`, `|`, `>`, `<`) in yellow, strings in cyan, and environment variables (`$VAR`, `$?`) in magenta.
- **💡 Fish-Style Autosuggestions** — Dimmed inline suggestions from your command history, accepted with the right arrow (`→`).
- **🔗 Command Chaining & Redirection** — Full support for conditional chaining (`&&`, `||`), sequential execution (`;`), background jobs (`&`), pipelines (`|`), and output/error redirection (`>`, `>>`, `<`, `2>`, `&>`).
- **📂 Smart Directory Jumper (`z`)** — Frecency-ranked navigation (`z <query>`) to jump straight to frequently and recently visited folders, plus an interactive numbered selector (`z -i [query]`).
- **🚀 Builtin `mkcd`** — Create directory hierarchies and enter them in a single command.
- **⚙️ Native Config & Dynamic Sourcing** — Reads `~/.bnbrc` on startup with fallback to `.zshrc`/`.bashrc` exports. Reload configurations on the fly with `source ~/.bnbrc`.
- **🧩 Rich Expansion Engine** — Wildcard globbing (`*`, `?`), brace expansion (`{1..5}`, `{a..e}`, `{foo,bar}`), exit code expansion (`$?`), parameter braces (`${VAR}`), process ID (`$$`), and tilde expansion (`~`).
- **📜 Non-Interactive Execution** — Run commands directly using `bnb -c "<command>"` or execute script files with `bnb <script>`.
- **✘ Exit Code Status Tracking** — Prompt arrow turns red (`❯`) and shows failing exit codes (`✘ 127`, `✘ 1`) after errors or interruptions (`Ctrl+C`).
- **🌐 Cross-Platform** — Runs natively on macOS, Linux, WSL, and Windows.

---

## 🚀 Installation

### Via Cargo (crates.io)

```bash
cargo install bnb-shell
```

### From Source

```bash
git clone https://github.com/EatSleepCode-Repeat/bnb.git
cd bnb
cargo install --path .
```

---

## 🛠 Usage

Launch the shell interactively:

```bash
bnb
```

Or execute a command non-interactively:

```bash
bnb -c "cargo check && cargo test"
```

To set `bnb` as your default login shell, add its binary path to `/etc/shells` and run:

```bash
chsh -s $(which bnb)
```

---

### Command-Line Flags

| Flag | Description |
| :--- | :--- |
| `-c <cmd>` | Execute a command string non-interactively and exit with its status code |
| `-h`, `--help` | Print usage information, available flags, and builtins |
| `-V`, `--version` | Print installed version information and exit |

---

### Builtin Commands

| Command | Description |
| :--- | :--- |
| `cd [dir \| -]` | Change directory (supports `cd -` to return to and print the previous directory) |
| `mkcd <dir>` | Create directory path (`mkdir -p`) and immediately switch into it |
| `z <term>` | Jump to the highest frecency matching directory |
| `z -i [term]` | List top ranked matches and choose one interactively |
| `which <cmd>` / `type <cmd>` | Inspect whether a command is a builtin, alias (with definition), or binary path |
| `alias [name=val]` | List all defined aliases or assign a new alias |
| `unalias <name \| -a>` | Remove a specific alias or all aliases (`-a`) |
| `export [key=val]` | Set environment variables, or list all exported variables when run with no arguments |
| `unset <key ...>` | Remove environment variables |
| `source <file>` / `. <file>` | Reload `~/.bnbrc` or execute an environment file in the current shell |
| `history [-c]` | Print numbered command history or clear history (`-c`) |
| `echo [-n] [-e] <text>` | Builtin echo supporting newline suppression (`-n`) and escape sequences (`-e`) |
| `pwd` | Print the current working directory |
| `clear` | Clear the terminal screen |
| `bnb-update` | Check crates.io and update `bnb-shell` in place |
| `exit [code]` | Exit the shell with an optional status code |

---

### Shell Syntax & Operators

- **Chaining**:
  ```bash
  cargo check && cargo test || echo "Build failed!"
  git add . && git commit -m "feat: new feature" && git push
  ```
- **Pipelines & Redirection**:
  ```bash
  alias | grep git
  echo "Hello, world" > output.txt
  cat < input.txt >> combined.txt
  ```
- **Brace & Range Expansion**:
  ```bash
  echo file_{1..5}.txt     # file_1.txt file_2.txt file_3.txt file_4.txt file_5.txt
  echo {a..c}.rs           # a.rs b.rs c.rs
  echo pre_{foo,bar}_post  # pre_foo_post pre_bar_post
  ```
- **Variable & Status Expansion**:
  ```bash
  false || echo "Exit code was: $?"
  echo "Process ID: $$"
  echo "${HOME}/projects"
  ```

---

### Configuration

On startup, `bnb` loads `~/.bnbrc` for aliases and environment variables. See [`.bnbrc.example`](.bnbrc.example) for an example configuration.

You can reload your configuration anytime without restarting:

```bash
source ~/.bnbrc
```

`bnb` checks crates.io once a day for a newer release and displays a notification if an update is available. You can update anytime with:

```bash
bnb-update
# or
cargo install bnb-shell --force
```

---

## 📜 License

MIT — see [LICENSE](LICENSE).
