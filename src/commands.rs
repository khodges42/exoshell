use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSuggestion {
    pub id: String,
    pub shell: CommandShell,
    pub command: String,
    pub explanation: Option<String>,
    pub model_risk: Option<RiskLevel>,
    pub detected_risk: CommandRisk,
    pub discarded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandShell {
    PowerShell,
    Posix,
    Unknown,
}

impl fmt::Display for CommandShell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PowerShell => formatter.write_str("powershell"),
            Self::Posix => formatter.write_str("posix"),
            Self::Unknown => formatter.write_str("unknown"),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CommandShellError {
    #[error("unknown command shell '{0}', expected powershell, posix, or unknown")]
    Unknown(String),
}

impl std::str::FromStr for CommandShell {
    type Err = CommandShellError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "powershell" | "pwsh" => Ok(Self::PowerShell),
            "posix" | "sh" | "bash" | "zsh" => Ok(Self::Posix),
            "unknown" => Ok(Self::Unknown),
            other => Err(CommandShellError::Unknown(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => formatter.write_str("low"),
            Self::Medium => formatter.write_str("medium"),
            Self::High => formatter.write_str("high"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRisk {
    pub level: RiskLevel,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRiskPolicy {
    pub include_defaults: bool,
    pub rules: Vec<CommandRiskRule>,
}

impl CommandRiskPolicy {
    pub fn evaluate(&self, command: &str, shell: CommandShell) -> CommandRisk {
        let command = command.to_ascii_lowercase();
        let mut reasons = Vec::new();

        let default_rules;
        let rules: Box<dyn Iterator<Item = &CommandRiskRule> + '_> = if self.include_defaults {
            default_rules = default_command_risk_rules();
            Box::new(default_rules.iter().chain(self.rules.iter()))
        } else {
            Box::new(self.rules.iter())
        };

        for rule in rules {
            if rule.matches(&command, shell) && !reasons.contains(&rule.reason) {
                reasons.push(rule.reason.clone());
            }
        }

        CommandRisk {
            level: if reasons.is_empty() {
                RiskLevel::Low
            } else {
                RiskLevel::High
            },
            reasons,
        }
    }
}

impl Default for CommandRiskPolicy {
    fn default() -> Self {
        Self {
            include_defaults: true,
            rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandRiskRule {
    pub match_all: Vec<String>,
    pub reason: String,
    #[serde(default)]
    pub shell: Option<CommandShell>,
}

impl CommandRiskRule {
    pub fn new(match_all: Vec<&str>, reason: &str, shell: Option<CommandShell>) -> Self {
        Self {
            match_all: match_all.into_iter().map(str::to_string).collect(),
            reason: reason.to_string(),
            shell,
        }
    }

    pub fn matches(&self, command: &str, shell: CommandShell) -> bool {
        if self.shell.is_some_and(|expected| expected != shell) {
            return false;
        }

        !self.match_all.is_empty()
            && self
                .match_all
                .iter()
                .all(|pattern| command.contains(&pattern.to_ascii_lowercase()))
    }
}

impl CommandRisk {
    pub fn warning(&self) -> Option<String> {
        if self.reasons.is_empty() {
            None
        } else {
            Some(format!("review required: {}", self.reasons.join("; ")))
        }
    }
}

#[cfg(test)]
fn parse_command_suggestions(response: &str) -> Vec<CommandSuggestion> {
    parse_command_suggestions_with_policy(response, &CommandRiskPolicy::default())
}

pub fn parse_command_suggestions_with_policy(
    response: &str,
    policy: &CommandRiskPolicy,
) -> Vec<CommandSuggestion> {
    let mut suggestions = Vec::new();
    let mut lines = response.lines().peekable();
    let mut previous_text = Vec::new();

    while let Some(line) = lines.next() {
        let Some(shell) = shell_from_fence(line) else {
            if !line.trim().is_empty() {
                previous_text.push(line.trim().to_string());
                if previous_text.len() > 3 {
                    previous_text.remove(0);
                }
            }
            continue;
        };

        let mut command_lines = Vec::new();
        for command_line in lines.by_ref() {
            if command_line.trim() == "```" {
                break;
            }
            command_lines.push(command_line);
        }

        let command = command_lines.join("\n").trim().to_string();
        if command.is_empty() {
            continue;
        }

        let id = format!("cmd-{:03}", suggestions.len() + 1);
        let model_risk = previous_text
            .iter()
            .rev()
            .find_map(|line| parse_risk_marker(line));
        let explanation = previous_text
            .iter()
            .rev()
            .find(|line| !line.contains("risk:") && !line.contains("[risk:"))
            .cloned();
        let detected_risk = detect_command_risk_with_policy(&command, shell, policy);

        suggestions.push(CommandSuggestion {
            id,
            shell,
            command,
            explanation,
            model_risk,
            detected_risk,
            discarded: false,
        });
    }

    suggestions
}

#[cfg(test)]
fn detect_command_risk(command: &str, shell: CommandShell) -> CommandRisk {
    detect_command_risk_with_policy(command, shell, &CommandRiskPolicy::default())
}

pub fn detect_command_risk_with_policy(
    command: &str,
    shell: CommandShell,
    policy: &CommandRiskPolicy,
) -> CommandRisk {
    policy.evaluate(command, shell)
}

pub fn render_suggestions(suggestions: &[CommandSuggestion]) -> String {
    if suggestions.is_empty() {
        return String::new();
    }

    let mut rendered = String::from("\nSuggested command actions:\n");
    for suggestion in suggestions {
        rendered.push_str(&format!(
            "- {} [{}]{}",
            suggestion.id,
            suggestion.shell,
            if suggestion.discarded {
                " discarded"
            } else {
                ""
            }
        ));
        if let Some(risk) = suggestion.model_risk {
            rendered.push_str(&format!(" model_risk={risk}"));
        }
        if let Some(warning) = suggestion.detected_risk.warning() {
            rendered.push_str(&format!(" warning=\"{warning}\""));
        }
        rendered.push('\n');
    }
    rendered.push_str(
        "Use /copy <id>, /explain <id>, or /discard <id>. Exoshell does not execute commands.",
    );
    rendered
}

fn shell_from_fence(line: &str) -> Option<CommandShell> {
    let language = line.trim().strip_prefix("```")?;
    match language {
        "powershell" | "pwsh" => Some(CommandShell::PowerShell),
        "sh" | "bash" | "zsh" | "posix" => Some(CommandShell::Posix),
        _ => None,
    }
}

fn parse_risk_marker(line: &str) -> Option<RiskLevel> {
    let lowered = line.to_ascii_lowercase();
    if lowered.contains("risk: high") || lowered.contains("[risk: high]") {
        Some(RiskLevel::High)
    } else if lowered.contains("risk: medium") || lowered.contains("[risk: medium]") {
        Some(RiskLevel::Medium)
    } else if lowered.contains("risk: low") || lowered.contains("[risk: low]") {
        Some(RiskLevel::Low)
    } else {
        None
    }
}

fn default_command_risk_rules() -> Vec<CommandRiskRule> {
    vec![
        CommandRiskRule::new(
            vec!["rm -rf"],
            "recursive or forced deletion",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["rm -fr"],
            "recursive or forced deletion",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["remove-item", "-recurse", "-force"],
            "recursive or forced deletion",
            Some(CommandShell::PowerShell),
        ),
        CommandRiskRule::new(vec!["del /s"], "recursive or forced deletion", None),
        CommandRiskRule::new(
            vec!["format-volume"],
            "disk formatting or partition operation",
            Some(CommandShell::PowerShell),
        ),
        CommandRiskRule::new(
            vec!["format "],
            "disk formatting or partition operation",
            None,
        ),
        CommandRiskRule::new(
            vec!["mkfs"],
            "disk formatting or partition operation",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["diskpart"],
            "disk formatting or partition operation",
            None,
        ),
        CommandRiskRule::new(
            vec!["chmod -r"],
            "recursive permission change",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["chown -r"],
            "recursive permission change",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["icacls ", "/grant"],
            "recursive permission change",
            Some(CommandShell::PowerShell),
        ),
        CommandRiskRule::new(
            vec!["curl ", "| sh"],
            "downloaded content piped to an interpreter",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["wget ", "| sh"],
            "downloaded content piped to an interpreter",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["irm ", "iex"],
            "downloaded content piped to an interpreter",
            Some(CommandShell::PowerShell),
        ),
        CommandRiskRule::new(
            vec!["invoke-restmethod", "invoke-expression"],
            "downloaded content piped to an interpreter",
            Some(CommandShell::PowerShell),
        ),
        CommandRiskRule::new(vec!["api_key"], "possible credential exposure", None),
        CommandRiskRule::new(vec!["apikey"], "possible credential exposure", None),
        CommandRiskRule::new(vec!["password"], "possible credential exposure", None),
        CommandRiskRule::new(vec!["secret"], "possible credential exposure", None),
        CommandRiskRule::new(vec!["token"], "possible credential exposure", None),
        CommandRiskRule::new(
            vec!["apt remove"],
            "package removal",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["apt purge"],
            "package removal",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["dnf remove"],
            "package removal",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["yum remove"],
            "package removal",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["pacman -r"],
            "package removal",
            Some(CommandShell::Posix),
        ),
        CommandRiskRule::new(
            vec!["uninstall-package"],
            "package removal",
            Some(CommandShell::PowerShell),
        ),
        CommandRiskRule::new(
            vec!["set-executionpolicy"],
            "PowerShell execution policy change",
            Some(CommandShell::PowerShell),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_fenced_command_suggestions() {
        let suggestions = parse_command_suggestions(
            "Inspect files:\n```sh\nrg TODO\n```\n[risk: high]\nRemove build:\n```powershell\nRemove-Item -Recurse -Force target\n```",
        );

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].id, "cmd-001");
        assert_eq!(suggestions[0].shell, CommandShell::Posix);
        assert_eq!(suggestions[0].command, "rg TODO");
        assert_eq!(suggestions[1].model_risk, Some(RiskLevel::High));
        assert_eq!(suggestions[1].detected_risk.level, RiskLevel::High);
    }

    #[test]
    fn invalid_or_empty_command_blocks_are_ignored() {
        let suggestions = parse_command_suggestions("```text\nnot a command\n```\n```sh\n\n```");

        assert!(suggestions.is_empty());
    }

    #[test]
    fn detects_representative_destructive_commands() {
        assert_eq!(
            detect_command_risk("rm -rf /tmp/example", CommandShell::Posix).level,
            RiskLevel::High
        );
        assert_eq!(
            detect_command_risk(
                "Invoke-RestMethod https://example.invalid/install.ps1 | Invoke-Expression",
                CommandShell::PowerShell
            )
            .level,
            RiskLevel::High
        );
        assert_eq!(
            detect_command_risk("rg TODO", CommandShell::Posix).level,
            RiskLevel::Low
        );
    }

    #[test]
    fn custom_policy_rules_extend_defaults() {
        let policy = CommandRiskPolicy {
            include_defaults: true,
            rules: vec![CommandRiskRule::new(
                vec!["kubectl delete", "--all"],
                "cluster-wide deletion",
                Some(CommandShell::Posix),
            )],
        };

        let risk = detect_command_risk_with_policy(
            "kubectl delete pods --all",
            CommandShell::Posix,
            &policy,
        );

        assert_eq!(risk.level, RiskLevel::High);
        assert_eq!(risk.reasons, vec!["cluster-wide deletion".to_string()]);
    }

    #[test]
    fn custom_policy_can_disable_defaults() {
        let policy = CommandRiskPolicy {
            include_defaults: false,
            rules: Vec::new(),
        };

        let risk = detect_command_risk_with_policy("rm -rf build", CommandShell::Posix, &policy);

        assert_eq!(risk.level, RiskLevel::Low);
        assert!(risk.reasons.is_empty());
    }

    #[test]
    fn parser_uses_supplied_policy() {
        let policy = CommandRiskPolicy {
            include_defaults: false,
            rules: vec![CommandRiskRule::new(
                vec!["terraform apply"],
                "infrastructure mutation",
                Some(CommandShell::Posix),
            )],
        };

        let suggestions = parse_command_suggestions_with_policy(
            "Apply infra:\n```sh\nterraform apply\n```",
            &policy,
        );

        assert_eq!(suggestions[0].detected_risk.level, RiskLevel::High);
        assert_eq!(
            suggestions[0].detected_risk.reasons,
            vec!["infrastructure mutation".to_string()]
        );
    }
}
