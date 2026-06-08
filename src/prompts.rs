use std::fmt;
use std::str::FromStr;

use crate::context::{ContextBudget, ContextEntry, ContextSize, render_prompt_context};
use crate::providers::{ChatMessage, ChatRole};
use crate::shell::ShellFamily;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    #[default]
    Operator,
    Audit,
    Teach,
    Quiet,
}

impl Stance {
    pub const ALL: [Stance; 4] = [Self::Operator, Self::Audit, Self::Teach, Self::Quiet];

    pub fn prompt_fragment(self) -> &'static str {
        match self {
            Self::Operator => {
                "Stance: operator.\n\
                 Favor concise, action-oriented help. Prioritize next steps, commands, and operational diagnosis. State uncertainty plainly without extended teaching. Mark risky suggestions clearly."
            }
            Self::Audit => {
                "Stance: audit.\n\
                 Prioritize security, correctness, reliability, and operational failure modes. Avoid mutating commands unless explicitly requested. Prioritize findings by severity when applicable. Distinguish evidence from inference."
            }
            Self::Teach => {
                "Stance: teach.\n\
                 Explain commands and concepts more fully while staying usable in a terminal. Explain flags and expected output when helpful. Do not assume shell-specific behavior is already known."
            }
            Self::Quiet => {
                "Stance: quiet.\n\
                 Minimize prose. Prefer direct commands and short rationale. Keep destructive-command and safety warnings visible."
            }
        }
    }

    pub fn names() -> String {
        Self::ALL
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for Stance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operator => formatter.write_str("operator"),
            Self::Audit => formatter.write_str("audit"),
            Self::Teach => formatter.write_str("teach"),
            Self::Quiet => formatter.write_str("quiet"),
        }
    }
}

impl FromStr for Stance {
    type Err = StanceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "operator" => Ok(Self::Operator),
            "audit" => Ok(Self::Audit),
            "teach" => Ok(Self::Teach),
            "quiet" => Ok(Self::Quiet),
            other => Err(StanceError::Unknown(other.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StanceError {
    #[error("unknown stance '{0}', expected operator, audit, teach, or quiet")]
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptBudgetEstimate {
    pub system: ContextSize,
    pub history: ContextSize,
    pub context: ContextSize,
    pub total: ContextSize,
    pub budget: ContextBudget,
}

impl PromptBudgetEstimate {
    pub fn is_context_over_budget(&self) -> bool {
        self.budget.is_over_budget(self.context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptAssembly {
    pub messages: Vec<ChatMessage>,
    pub estimate: PromptBudgetEstimate,
}

pub fn phase2_system_prompt(shell_family: ShellFamily, stance: Stance) -> String {
    format!(
        "You are Exoshell, a shell-adjacent assistant for technical operators.\n\
         Sacred rule: enhance skill; do not replace it.\n\
         The human keeps the controls. Suggest commands, but never say or imply that you executed them.\n\
         Do not request or perform autonomous execution. Do not normalize blind destructive commands.\n\
         Context is explicit, operator-selected, and session-scoped. Treat attached context as visible working material, not hidden memory.\n\
         Prefer deterministic shell tools when they are the right fit; do not force AI into workflows where awk, sed, jq, rg, git, cargo, make, or platform-native tools are better.\n\
         Surface uncertainty with signal language such as 'signal: weak', 'signal: medium', or 'signal: high' when confidence matters.\n\
         Target shell family: {shell_family}.\n\
         Shell instructions: {}\n\
         Command convention: put suggested commands in fenced code blocks using language tag `{}`. Add a short review note before risky operations. For destructive commands, require explicit user review and provide a safer inspection command first when possible.\n\
         {}",
        shell_family.prompt_instructions(),
        command_language(shell_family),
        stance.prompt_fragment()
    )
}

pub fn assemble_prompt(
    shell_family: ShellFamily,
    stance: Stance,
    conversation: &[ChatMessage],
    context_entries: &[ContextEntry],
    budget: ContextBudget,
) -> PromptAssembly {
    let system_prompt = phase2_system_prompt(shell_family, stance);
    let rendered_context = render_prompt_context(context_entries);
    let estimate = estimate_prompt(&system_prompt, conversation, &rendered_context, budget);

    let mut messages = Vec::with_capacity(conversation.len() + 2);
    messages.push(ChatMessage::new(ChatRole::System, system_prompt));

    if rendered_context.is_empty() {
        messages.extend_from_slice(conversation);
    } else {
        let insert_at = conversation.len().saturating_sub(1);
        messages.extend_from_slice(&conversation[..insert_at]);
        messages.push(ChatMessage::new(
            ChatRole::User,
            format!(
                "Explicit session context selected by the operator follows.\n\n{}",
                rendered_context
            ),
        ));
        messages.extend_from_slice(&conversation[insert_at..]);
    }

    PromptAssembly { messages, estimate }
}

pub fn estimate_prompt(
    system_prompt: &str,
    conversation: &[ChatMessage],
    rendered_context: &str,
    budget: ContextBudget,
) -> PromptBudgetEstimate {
    let system = ContextSize::from_content(system_prompt);
    let history = ContextSize::from_content(
        &conversation
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let context = ContextSize::from_content(rendered_context);
    let total = ContextSize {
        characters: system.characters + history.characters + context.characters,
        estimated_tokens: system.estimated_tokens
            + history.estimated_tokens
            + context.estimated_tokens,
    };

    PromptBudgetEstimate {
        system,
        history,
        context,
        total,
        budget,
    }
}

pub fn render_prompt_estimate(estimate: PromptBudgetEstimate) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!(
        "system: {} chars / ~{} tokens\n",
        estimate.system.characters, estimate.system.estimated_tokens
    ));
    rendered.push_str(&format!(
        "history: {} chars / ~{} tokens\n",
        estimate.history.characters, estimate.history.estimated_tokens
    ));
    rendered.push_str(&format!(
        "attached_context: {} chars / ~{} tokens\n",
        estimate.context.characters, estimate.context.estimated_tokens
    ));
    rendered.push_str(&format!(
        "estimated_total: {} chars / ~{} tokens",
        estimate.total.characters, estimate.total.estimated_tokens
    ));
    if let Some(max) = estimate.budget.max_characters {
        rendered.push_str(&format!(
            "\ncontext_character_budget: {}/{}",
            estimate.context.characters, max
        ));
    }
    if let Some(max) = estimate.budget.max_estimated_tokens {
        rendered.push_str(&format!(
            "\ncontext_token_budget: {}/{}",
            estimate.context.estimated_tokens, max
        ));
    }
    if estimate.is_context_over_budget() {
        rendered.push_str("\nwarning: attached context exceeds configured budget");
    }

    rendered
}

fn command_language(shell_family: ShellFamily) -> &'static str {
    match shell_family {
        ShellFamily::PowerShell => "powershell",
        ShellFamily::Posix => "sh",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextEntry, ContextKind, ContextProvenance};

    #[test]
    fn stance_names_parse_and_display() {
        assert_eq!("audit".parse::<Stance>().expect("stance"), Stance::Audit);
        assert_eq!(Stance::names(), "operator, audit, teach, quiet");
        assert!("loud".parse::<Stance>().is_err());
    }

    #[test]
    fn system_prompt_contains_stance_and_shell_specific_instructions() {
        let prompt = phase2_system_prompt(ShellFamily::PowerShell, Stance::Audit);

        assert!(prompt.contains("Target shell family: powershell"));
        assert!(prompt.contains("Use PowerShell syntax"));
        assert!(prompt.contains("Stance: audit"));
        assert!(prompt.contains("Distinguish evidence from inference"));
        assert!(prompt.contains("never say or imply that you executed"));
        assert!(prompt.contains("fenced code blocks using language tag `powershell`"));
    }

    #[test]
    fn quiet_stance_keeps_safety_language() {
        let prompt = phase2_system_prompt(ShellFamily::Posix, Stance::Quiet);

        assert!(prompt.contains("Stance: quiet"));
        assert!(prompt.contains("Keep destructive-command and safety warnings visible"));
        assert!(prompt.contains("fenced code blocks using language tag `sh`"));
    }

    #[test]
    fn prompt_assembly_orders_system_history_context_and_current_input() {
        let context = vec![ContextEntry::new(
            "ctx-001",
            ContextKind::Manual,
            "note",
            ContextProvenance::manual(),
            "repo uses cargo",
        )];
        let conversation = vec![
            ChatMessage::new(ChatRole::User, "first"),
            ChatMessage::new(ChatRole::Assistant, "answer"),
            ChatMessage::new(ChatRole::User, "current"),
        ];

        let assembly = assemble_prompt(
            ShellFamily::Posix,
            Stance::Operator,
            &conversation,
            &context,
            ContextBudget::default(),
        );

        assert_eq!(assembly.messages[0].role, ChatRole::System);
        assert!(assembly.messages[0].content.contains("Stance: operator"));
        assert_eq!(assembly.messages[1].content, "first");
        assert_eq!(assembly.messages[2].content, "answer");
        assert!(assembly.messages[3].content.contains("[Context: ctx-001]"));
        assert_eq!(assembly.messages[4].content, "current");
        assert!(assembly.estimate.total.characters > assembly.estimate.context.characters);
    }

    #[test]
    fn prompt_estimate_reports_context_budget_warning() {
        let estimate = estimate_prompt(
            "system",
            &[ChatMessage::new(ChatRole::User, "hello")],
            "large context",
            ContextBudget {
                max_characters: Some(3),
                max_estimated_tokens: None,
            },
        );

        assert!(render_prompt_estimate(estimate).contains("warning"));
    }
}
