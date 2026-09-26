# ⚡ bnb-shell

A fast, polished, cross-platform terminal shell built in Rust featuring a two-line Powerlevel10k aesthetic, smart directory jumping, syntax highlighting, and history suggestions.

---

## ✨ Features

- **🎨 Powerlevel10k-Style Prompt:** Dynamic 2-line prompt with OS badges (``, ``, ``), directory pathing, Git branch integration, and right-aligned execution duration badges (`took 2s ⌛`).
- **⚡ Real-Time Syntax Highlighting:** Valid system commands and builtins glow **green**, invalid commands turn **red**, operators (`|`, `>`, `<`) glow **yellow**, and environment variables (`$VAR`) display in **magenta**.
- **💡 Fish-Style Auto-Suggestions:** History completions appear as dimmed ghost text ahead of your cursor as you type.
- **📂 Smart Directory Jumper (`z`):** Built-in frecency-based folder navigation. Jump straight to frequently visited directories with `z <search_term>`.
- **✘ Exit Code Status Tracking:** Prompt arrow turns **red** (`❯`) and displays error status badges (`✘ 127` / `✘ 1`) when commands fail.
- **⚙️ Native `.bnbrc` & `.zshrc` Integration:** Reads environment variables (`export`) and aliases natively, with automatic fallback execution for custom shell functions.
- **🌐 Cross-Platform:** Designed to run seamlessly across macOS, Linux/WSL, and Windows.

---

## 🚀 Installation

### Via Cargo (crates.io)
```bash
cargo install bnb-shell
```

### From Source
```bash
git clone [https://github.com/EatSleepCode-Repeat/bnb.git](https://github.com/EatSleepCode-Repeat/bnb.git)
cd bnb
cargo install --path .
```

---

## 🛠 Usage

Launch the shell from any terminal:

```bash
bnb
```

### Builtin Commands

| Command | Description |
| :--- | :--- |
| `z <dir>` | Jump to the highest frecency matching folder |
| `cd <dir>` | Change directory |
| `alias` | List or set active command aliases |
| `export` | Set or inspect environment variables |
| `exit` | Exit the shell |

---

## 📜 License

Distributed under the MIT License.
