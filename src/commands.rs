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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl CommandRisk {
    pub fn none() -> Self {
        Self {
            level: RiskLevel::Low,
            reasons: Vec::new(),
        }
    }

    pub fn warning(&self) -> Option<String> {
        if self.reasons.is_empty() {
            None
        } else {
            Some(format!(
                "review required: {}",
                self.reasons.join("; ")
            ))
        }
    }
}

pub fn parse_command_suggestions(response: &str) -> Vec<CommandSuggestion> {
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
        let detected_risk = detect_command_risk(&command, shell);

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

pub fn detect_command_risk(command: &str, shell: CommandShell) -> CommandRisk {
    let lowered = command.to_ascii_lowercase();
    let mut reasons = Vec::new();

    if lowered.contains("rm -rf")
        || lowered.contains("rm -fr")
        || lowered.contains("remove-item") && lowered.contains("-recurse") && lowered.contains("-force")
        || lowered.contains("del /s")
    {
        reasons.push("recursive or forced deletion".into());
    }
    if lowered.contains("format-volume")
        || lowered.contains("format ")
        || lowered.contains("mkfs")
        || lowered.contains("diskpart")
    {
        reasons.push("disk formatting or partition operation".into());
    }
    if lowered.contains("chmod -r")
        || lowered.contains("chown -r")
        || lowered.contains("icacls ") && lowered.contains("/grant")
    {
        reasons.push("recursive permission change".into());
    }
    if lowered.contains("curl ") && lowered.contains("| sh")
        || lowered.contains("wget ") && lowered.contains("| sh")
        || lowered.contains("irm ") && lowered.contains("iex")
        || lowered.contains("invoke-restmethod") && lowered.contains("invoke-expression")
    {
        reasons.push("downloaded content piped to an interpreter".into());
    }
    if lowered.contains("api_key")
        || lowered.contains("apikey")
        || lowered.contains("password")
        || lowered.contains("secret")
        || lowered.contains("token")
    {
        reasons.push("possible credential exposure".into());
    }
    if lowered.contains("apt remove")
        || lowered.contains("apt purge")
        || lowered.contains("dnf remove")
        || lowered.contains("yum remove")
        || lowered.contains("pacman -r")
        || lowered.contains("uninstall-package")
    {
        reasons.push("package removal".into());
    }

    if shell == CommandShell::PowerShell && lowered.contains("set-executionpolicy") {
        reasons.push("PowerShell execution policy change".into());
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
            if suggestion.discarded { " discarded" } else { "" }
        ));
        if let Some(risk) = suggestion.model_risk {
            rendered.push_str(&format!(" model_risk={risk}"));
        }
        if let Some(warning) = suggestion.detected_risk.warning() {
            rendered.push_str(&format!(" warning=\"{warning}\""));
        }
        rendered.push('\n');
    }
    rendered.push_str("Use /copy <id>, /explain <id>, or /discard <id>. Exoshell does not execute commands.");
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
}
