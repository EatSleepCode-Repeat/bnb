use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use terminal_size::{terminal_size, Width};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorScheme {
    Classic,
    Catppuccin,
    Dracula,
    Nord,
    Gruvbox,
    TokyoNight,
}

impl ColorScheme {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "catppuccin" => Self::Catppuccin,
            "dracula" => Self::Dracula,
            "nord" => Self::Nord,
            "gruvbox" => Self::Gruvbox,
            "tokyo-night" | "tokyonight" => Self::TokyoNight,
            _ => Self::Classic,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Catppuccin => "catppuccin",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::Gruvbox => "gruvbox",
            Self::TokyoNight => "tokyo-night",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Catppuccin => "Catppuccin Mocha",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::Gruvbox => "Gruvbox Dark",
            Self::TokyoNight => "Tokyo Night",
        }
    }

    // Returns ANSI 256-color numbers: (os_bg, path_bg, git_clean_bg, git_dirty_bg)
    pub fn palette_256(&self) -> (u8, u8, u8, u8) {
        match self {
            Self::Classic => (238, 75, 141, 178),
            Self::Catppuccin => (237, 111, 114, 215),
            Self::Dracula => (236, 117, 84, 215),
            Self::Nord => (237, 110, 108, 221),
            Self::Gruvbox => (237, 108, 142, 208),
            Self::TokyoNight => (236, 75, 120, 221),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PromptConfig {
    pub style: String,             // "powerline", "lean", "minimal"
    pub separator: String,         // "angled", "rounded", "slanted", "none"
    pub enable_icons: bool,        // true / false
    pub symbol: String,            // "❯", "➜", "$", "λ"
    pub color_scheme: ColorScheme, // classic, catppuccin, dracula, nord, etc.
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            style: "powerline".to_string(),
            separator: "angled".to_string(),
            enable_icons: true,
            symbol: "❯".to_string(),
            color_scheme: ColorScheme::Classic,
        }
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".bnb_prompt.conf"))
}

pub fn load_prompt_config() -> PromptConfig {
    let mut config = PromptConfig::default();
    let path = match get_config_path() {
        Some(p) => p,
        None => return config,
    };

    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some((k, v)) = trimmed.split_once('=') {
                match k.trim() {
                    "style" => config.style = v.trim().to_string(),
                    "separator" => config.separator = v.trim().to_string(),
                    "enable_icons" => config.enable_icons = v.trim().parse().unwrap_or(true),
                    "symbol" => config.symbol = v.trim().to_string(),
                    "color_scheme" => config.color_scheme = ColorScheme::from_str(v.trim()),
                    _ => {}
                }
            }
        }
    }

    config
}

pub fn save_prompt_config(config: &PromptConfig) -> Result<(), String> {
    let path = get_config_path().ok_or_else(|| "cannot find home directory".to_string())?;
    let content = format!(
        "style={}\nseparator={}\nenable_icons={}\nsymbol={}\ncolor_scheme={}\n",
        config.style,
        config.separator,
        config.enable_icons,
        config.symbol,
        config.color_scheme.key()
    );
    fs::write(path, content).map_err(|e| format!("cannot save prompt config: {}", e))
}

struct GitStatus {
    branch: String,
    dirty_count: usize,
    untracked_count: usize,
    ahead: u32,
    behind: u32,
}

pub fn get_prompt(last_duration: Option<Duration>, last_status: Option<i32>) -> String {
    let config = load_prompt_config();
    render_prompt_with_config(&config, last_duration, last_status)
}

pub fn render_prompt_with_config(
    config: &PromptConfig,
    last_duration: Option<Duration>,
    last_status: Option<i32>,
) -> String {
    let cwd = env::current_dir()
        .map(|p| {
            let path = p.display().to_string();
            if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
                if path.starts_with(&home) {
                    return path.replacen(&home, "~", 1);
                }
            }
            path
        })
        .unwrap_or_else(|_| "/".into());

    let git_status = get_git_status();

    let os_icon = if !config.enable_icons {
        "OS"
    } else if cfg!(target_os = "macos") {
        ""
    } else if cfg!(target_os = "linux") {
        ""
    } else {
        ""
    };

    let dir_icon = if config.enable_icons { "📁 " } else { "" };
    let git_icon = if config.enable_icons { " " } else { "git:" };

    let sep_char = match config.separator.as_str() {
        "rounded" => "\u{e0b4}",
        "slanted" => "\u{e0bc}",
        "none" => " ",
        _ => "\u{e0b0}", // angled
    };

    let left_bar = match config.style.as_str() {
        "lean" => {
            let os_part = format!("\x1b[1;36m{}\x1b[0m", os_icon);
            let dir_part = format!("\x1b[1;34m{}{}\x1b[0m", dir_icon, cwd);
            let git_part = match &git_status {
                Some(gs) => {
                    let is_dirty = gs.dirty_count > 0 || gs.untracked_count > 0;
                    let git_color = if is_dirty { "\x1b[1;33m" } else { "\x1b[1;35m" };
                    let mut flags = String::new();
                    if gs.dirty_count > 0 {
                        flags.push_str(&format!(" !{}", gs.dirty_count));
                    }
                    if gs.untracked_count > 0 {
                        flags.push_str(&format!(" ?{}", gs.untracked_count));
                    }
                    if gs.ahead > 0 {
                        flags.push_str(&format!(" ⇡{}", gs.ahead));
                    }
                    if gs.behind > 0 {
                        flags.push_str(&format!(" ⇣{}", gs.behind));
                    }
                    format!(" {}{}{}{}\x1b[0m", git_color, git_icon, gs.branch, flags)
                }
                None => String::new(),
            };
            format!("{} {} {}", os_part, dir_part, git_part)
        }
        "minimal" => {
            let git_part = match &git_status {
                Some(gs) => format!(" \x1b[1;33m({})\x1b[0m", gs.branch),
                None => String::new(),
            };
            format!("\x1b[1;32m{}\x1b[0m{}", cwd, git_part)
        }
        _ => {
            // Powerline (Theme-aware)
            let (os_bg_num, path_bg_num, git_clean_num, git_dirty_num) =
                config.color_scheme.palette_256();

            let seg1_bg = format!("\x1b[48;5;{}m", os_bg_num);
            let seg1_fg = "\x1b[1;38;5;255m";
            let seg1_text = format!("{}{} {}  ", seg1_bg, seg1_fg, os_icon);

            let seg2_bg = format!("\x1b[48;5;{}m", path_bg_num);
            let seg2_fg = "\x1b[1;38;5;255m";
            let trans1 = format!("\x1b[38;5;{}m{}{}", os_bg_num, seg2_bg, sep_char);
            let seg2_text = format!("{} {}{} ", seg2_fg, dir_icon, cwd);

            let (seg3_text, last_bg_fg) = match &git_status {
                Some(gs) => {
                    let is_dirty = gs.dirty_count > 0 || gs.untracked_count > 0;
                    let (git_bg_num, git_bg_code) = if is_dirty {
                        (git_dirty_num, format!("\x1b[48;5;{}m", git_dirty_num))
                    } else {
                        (git_clean_num, format!("\x1b[48;5;{}m", git_clean_num))
                    };
                    let git_fg = "\x1b[1;38;5;232m";
                    let trans2 = format!("\x1b[38;5;{}m{}{}", path_bg_num, git_bg_code, sep_char);

                    let mut flags = String::new();
                    if gs.dirty_count > 0 {
                        flags.push_str(&format!(" !{}", gs.dirty_count));
                    }
                    if gs.untracked_count > 0 {
                        flags.push_str(&format!(" ?{}", gs.untracked_count));
                    }
                    if gs.ahead > 0 {
                        flags.push_str(&format!(" ⇡{}", gs.ahead));
                    }
                    if gs.behind > 0 {
                        flags.push_str(&format!(" ⇣{}", gs.behind));
                    }

                    let git_content = format!(
                        "{}{} {}{}{} ",
                        git_bg_code, git_fg, git_icon, gs.branch, flags
                    );
                    (
                        format!("{}{}", trans2, git_content),
                        format!("\x1b[38;5;{}m", git_bg_num),
                    )
                }
                None => (String::new(), format!("\x1b[38;5;{}m", path_bg_num)),
            };

            let left_bar_end = format!("\x1b[0m{}{}\x1b[0m", last_bg_fg, sep_char);
            format!(
                "{}{}{}{}{}",
                seg1_text, trans1, seg2_text, seg3_text, left_bar_end
            )
        }
    };

    let status_badge = match last_status {
        Some(code) if code != 0 => {
            format!("\x1b[48;5;196m\x1b[1;38;5;255m ✘ {} \x1b[0m", code)
        }
        _ => String::new(),
    };

    let duration_badge = match last_duration {
        Some(d) if d.as_millis() >= 500 => {
            let duration_str = format_duration(d);
            format!(
                "\x1b[48;5;215m\x1b[1;38;5;232m took {} ⌛ \x1b[0m",
                duration_str
            )
        }
        _ => String::new(),
    };

    let right_badges = match (!status_badge.is_empty(), !duration_badge.is_empty()) {
        (true, true) => format!("{} {}", status_badge, duration_badge),
        (true, false) => status_badge,
        (false, true) => duration_badge,
        (false, false) => String::new(),
    };

    let term_width = terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80);

    let left_len = strip_ansi(&left_bar).chars().count() + 1;
    let right_len = if right_badges.is_empty() {
        0
    } else {
        strip_ansi(&right_badges).chars().count()
            + (if right_badges.contains('⌛') { 1 } else { 0 })
    };

    let line1 = if !right_badges.is_empty() && term_width > (left_len + right_len + 2) {
        let spaces_count = term_width.saturating_sub(left_len + right_len + 1);
        let spaces = " ".repeat(spaces_count);
        format!("{}{}{}", left_bar, spaces, right_badges)
    } else {
        format!("{}{}", left_bar, right_badges)
    };

    let symbol = if config.symbol.is_empty() {
        "❯"
    } else {
        &config.symbol
    };
    let prompt_arrow = if last_status.unwrap_or(0) != 0 {
        format!("\x1b[1;31m{}\x1b[0m ", symbol)
    } else {
        format!("\x1b[1;32m{}\x1b[0m ", symbol)
    };

    format!("{}\n{}", line1, prompt_arrow)
}

fn strip_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for c in input.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' || c == 'K' || c == 'H' || c == 'J' {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn get_git_status() -> Option<GitStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-b"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    let branch_line = lines.next()?;

    if !branch_line.starts_with("## ") {
        return None;
    }

    let branch_part = &branch_line[3..];
    let (branch_name, ahead, behind) = parse_branch_header(branch_part);

    let mut dirty_count = 0;
    let mut untracked_count = 0;

    for line in lines {
        if line.starts_with("??") {
            untracked_count += 1;
        } else if !line.trim().is_empty() {
            dirty_count += 1;
        }
    }

    Some(GitStatus {
        branch: branch_name,
        dirty_count,
        untracked_count,
        ahead,
        behind,
    })
}

fn parse_branch_header(header: &str) -> (String, u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;

    let branch_spec = if let Some((name_part, flags_part)) = header.split_once(" [") {
        let flags = flags_part.trim_end_matches(']');
        for part in flags.split(',') {
            let trimmed = part.trim();
            if let Some(num_str) = trimmed.strip_prefix("ahead ") {
                ahead = num_str.parse().unwrap_or(0);
            } else if let Some(num_str) = trimmed.strip_prefix("behind ") {
                behind = num_str.parse().unwrap_or(0);
            }
        }
        name_part
    } else {
        header
    };

    let branch_name = if let Some((local, _)) = branch_spec.split_once("...") {
        local.to_string()
    } else if branch_spec.starts_with("HEAD (no branch") {
        ":detached".to_string()
    } else {
        branch_spec.to_string()
    };

    (branch_name, ahead, behind)
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 60 {
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{}m {}s", mins, rem_secs)
    } else if secs >= 10 {
        format!("{}s", secs)
    } else if secs >= 1 {
        let tenths = d.subsec_millis() / 100;
        format!("{}.{}s", secs, tenths)
    } else {
        format!("{}ms", d.as_millis())
    }
}

pub fn load_history_entries() -> Vec<String> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let history_path = home.join(".bnb_history");
    if !history_path.exists() {
        return Vec::new();
    }

    fs::read_to_string(history_path)
        .map(|data| data.lines().rev().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}
