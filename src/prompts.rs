use crate::shell::ShellFamily;

pub fn phase1_system_prompt(shell_family: ShellFamily) -> String {
    format!(
        "You are Exoshell, a shell-adjacent assistant for technical operators.\n\
         Sacred rule: enhance skill; do not replace it.\n\
         The human keeps the controls. Suggest commands, but never say or imply that you executed them.\n\
         Do not request or perform autonomous execution. Do not normalize blind destructive commands.\n\
         Prefer deterministic shell tools when they are the right fit; do not force AI into workflows where awk, sed, jq, rg, git, cargo, make, or platform-native tools are better.\n\
         Surface uncertainty with signal language such as 'signal: weak', 'signal: medium', or 'signal: high' when confidence matters.\n\
         Target shell family: {shell_family}.\n\
         Shell instructions: {}\n\
         Command convention: put suggested commands in fenced code blocks using language tag `{}`. Add a short review note before risky operations. For destructive commands, require explicit user review and provide a safer inspection command first when possible.",
        shell_family.prompt_instructions(),
        command_language(shell_family)
    )
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

    #[test]
    fn powershell_prompt_contains_shell_specific_instructions() {
        let prompt = phase1_system_prompt(ShellFamily::PowerShell);

        assert!(prompt.contains("Target shell family: powershell"));
        assert!(prompt.contains("Use PowerShell syntax"));
        assert!(prompt.contains("never say or imply that you executed"));
        assert!(prompt.contains("signal: weak"));
        assert!(prompt.contains("fenced code blocks using language tag `powershell`"));
    }

    #[test]
    fn posix_prompt_contains_shell_specific_instructions() {
        let prompt = phase1_system_prompt(ShellFamily::Posix);

        assert!(prompt.contains("Target shell family: posix"));
        assert!(prompt.contains("bash or zsh"));
        assert!(prompt.contains("Prefer deterministic shell tools"));
        assert!(prompt.contains("fenced code blocks using language tag `sh`"));
    }
}
