use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFamily {
    PowerShell,
    Posix,
}

impl ShellFamily {
    pub fn default_for_platform() -> Self {
        if cfg!(windows) {
            Self::PowerShell
        } else {
            Self::Posix
        }
    }

    pub fn prompt_instructions(self) -> &'static str {
        match self {
            Self::PowerShell => {
                "Use PowerShell syntax. Prefer PowerShell cmdlets for native Windows tasks, and use external tools such as rg, git, jq, or cargo when they are the right deterministic tool."
            }
            Self::Posix => {
                "Use POSIX-like shell syntax suitable for bash or zsh. Prefer standard shell tools and common deterministic utilities such as rg, awk, sed, jq, git, make, or cargo when they fit."
            }
        }
    }
}

impl fmt::Display for ShellFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PowerShell => formatter.write_str("powershell"),
            Self::Posix => formatter.write_str("posix"),
        }
    }
}

impl FromStr for ShellFamily {
    type Err = ShellFamilyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "powershell" => Ok(Self::PowerShell),
            "posix" => Ok(Self::Posix),
            other => Err(ShellFamilyError {
                value: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unsupported shell family '{value}', expected 'powershell' or 'posix'")]
pub struct ShellFamilyError {
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_shell_families() {
        assert_eq!(
            "powershell".parse::<ShellFamily>().expect("powershell"),
            ShellFamily::PowerShell
        );
        assert_eq!(
            "posix".parse::<ShellFamily>().expect("posix"),
            ShellFamily::Posix
        );
    }

    #[test]
    fn rejects_unsupported_shell_family() {
        assert!("cmd".parse::<ShellFamily>().is_err());
    }
}
