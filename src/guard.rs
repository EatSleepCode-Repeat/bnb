use regex::Regex;
use std::io::{self, Write};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedSecret {
    pub pattern_name: &'static str,
    pub raw: String,
    pub masked: String,
    pub suggested_name: String,
}

static PATTERNS: OnceLock<Vec<(&'static str, &'static str, Regex)>> = OnceLock::new();

fn get_patterns() -> &'static Vec<(&'static str, &'static str, Regex)> {
    PATTERNS.get_or_init(|| {
        vec![
            (
                "GitHub Token",
                "gh_token",
                Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36}").unwrap(),
            ),
            (
                "OpenAI / Anthropic Key",
                "ai_key",
                Regex::new(r"sk-(?:proj-|ant-)?[A-Za-z0-9_\-]{20,}").unwrap(),
            ),
            (
                "AWS Access Key ID",
                "aws_key_id",
                Regex::new(r"(?:AKIA|ASIA|AROA|AIPA)[A-Z0-9]{16}").unwrap(),
            ),
            (
                "Stripe API Key",
                "stripe_key",
                Regex::new(r"[sr]k_(?:test|live)_[0-9a-zA-Z]{24,}").unwrap(),
            ),
            (
                "Slack Token",
                "slack_token",
                Regex::new(r"xox[baprs]-[0-9a-zA-Z]{10,48}").unwrap(),
            ),
        ]
    })
}

/// Scans an input command for known credential formats.
pub fn scan_command(command: &str) -> Vec<DetectedSecret> {
    let mut detected = Vec::new();

    for (label, default_name, re) in get_patterns() {
        for mat in re.find_iter(command) {
            let raw = mat.as_str().to_string();

            // Mask all characters except the first 4 and last 3
            let masked = if raw.len() > 8 {
                format!("{}...{}", &raw[..4], &raw[raw.len() - 3..])
            } else {
                "***".to_string()
            };

            detected.push(DetectedSecret {
                pattern_name: label,
                raw,
                masked,
                suggested_name: default_name.to_string(),
            });
        }
    }

    detected
}

pub enum GuardAction {
    Execute(String), // Command string (rewritten or original)
    Abort,           // User discarded the command
}

/// Intercepts commands containing secrets, prompting to vault them before execution.
pub fn intercept_if_secret<F>(
    raw_command: &str,
    mut store_in_vault: F,
) -> GuardAction
where
    F: FnMut(&str, &str) -> Result<(), String>,
{
    let secrets = scan_command(raw_command);
    if secrets.is_empty() {
        return GuardAction::Execute(raw_command.to_string());
    }

    println!("\x1b[1;33m\n⚠️  bnb Secret Guard:\x1b[0m Potential credential detected in input!");

    let mut current_command = raw_command.to_string();

    for (idx, secret) in secrets.iter().enumerate() {
        println!(
            "  [\x1b[1m{}\x1b[0m] \x1b[36m{}\x1b[0m: \x1b[1m{}\x1b[0m",
            idx + 1,
            secret.pattern_name,
            secret.masked
        );
        print!("  Save to bnb vault? [\x1b[32mY\x1b[0mes / \x1b[33mi\x1b[0mgnore / \x1b[31mc\x1b[0mancel] (default: Y): ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim().to_lowercase();

        if choice == "c" || choice == "cancel" {
            println!("\x1b[31mCommand aborted. Plaintext credentials were not executed or logged.\x1b[0m\n");
            return GuardAction::Abort;
        }

        if choice == "i" || choice == "ignore" {
            println!("\x1b[33mPassing credential through as plaintext...\x1b[0m");
            continue;
        }

        // Default to "yes"
        print!("  Vault key name (default: '{}'): ", secret.suggested_name);
        io::stdout().flush().unwrap();

        let mut key_input = String::new();
        io::stdin().read_line(&mut key_input).unwrap();
        let key_name = match key_input.trim() {
            "" => secret.suggested_name.clone(),
            custom => custom.to_string(),
        };

        match store_in_vault(&key_name, &secret.raw) {
            Ok(_) => {
                let placeholder = format!("$vault:{}", key_name);
                current_command = current_command.replace(&secret.raw, &placeholder);
                println!(
                    "  \x1b[32m✓\x1b[0m Stored as '\x1b[1m{}\x1b[0m' and rewritten to: \x1b[2m{}\x1b[0m",
                    key_name, placeholder
                );
            }
            Err(err) => {
                println!("\x1b[31m  ✗ Vault storage failed: {}\x1b[0m", err);
            }
        }
    }

    println!();
    GuardAction::Execute(current_command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_token_detection() {
        let input = "curl -H 'Authorization: token ghp_111122223333444455556666777788889999' https://api.github.com";
        let findings = scan_command(input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern_name, "GitHub Token");
        assert_eq!(findings[0].raw, "ghp_111122223333444455556666777788889999");
        assert_eq!(findings[0].masked, "ghp_...999");
    }

    #[test]
    fn test_clean_command_no_findings() {
        let input = "cargo build --release && ls -la";
        let findings = scan_command(input);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aws_key_detection() {
        let input = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let findings = scan_command(input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern_name, "AWS Access Key ID");
        assert_eq!(findings[0].suggested_name, "aws_key_id");
    }
}