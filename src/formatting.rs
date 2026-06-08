use crate::commands::{
    CommandRiskPolicy, parse_command_suggestions_with_policy, render_suggestions,
};

#[cfg(test)]
fn render_assistant_output(response: &str) -> String {
    render_assistant_output_with_policy(response, &CommandRiskPolicy::default())
}

pub fn render_assistant_output_with_policy(response: &str, policy: &CommandRiskPolicy) -> String {
    let mut rendered = String::new();
    let mut in_command_block = false;

    for line in response.lines() {
        if in_command_block && line.trim() == "```" {
            in_command_block = false;
            rendered.push_str("--- end suggested command ---\n");
            rendered.push_str(line);
            rendered.push('\n');
            continue;
        }

        if let Some(language) = command_language_from_fence(line) {
            rendered.push_str(&format!(
                "--- suggested command ({language}); review before running ---\n"
            ));
            in_command_block = true;
            rendered.push_str(line);
            rendered.push('\n');
            continue;
        }

        rendered.push_str(line);
        rendered.push('\n');
    }

    let suggestions = parse_command_suggestions_with_policy(response, policy);
    if !suggestions.is_empty() {
        rendered.push_str(&render_suggestions(&suggestions));
        rendered.push('\n');
    }

    if response.contains("[risk: high]") || response.contains("risk: high") {
        rendered.push_str("--- review required: model marked this response as high risk ---\n");
    }

    rendered.trim_end().to_string()
}

fn command_language_from_fence(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let language = trimmed.strip_prefix("```")?;

    match language {
        "powershell" | "pwsh" | "sh" | "bash" | "zsh" | "posix" => Some(language),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_command_blocks() {
        let output = render_assistant_output(
            "Try this:\n```powershell\nGet-ChildItem\n```\nThen inspect the output.",
        );

        assert!(output.contains("suggested command (powershell); review before running"));
        assert!(output.contains("end suggested command"));
    }

    #[test]
    fn preserves_high_risk_language() {
        let output = render_assistant_output("[risk: high]\n```sh\nrm -rf build\n```");

        assert!(output.contains("review required"));
        assert!(output.contains("rm -rf build"));
    }
}
