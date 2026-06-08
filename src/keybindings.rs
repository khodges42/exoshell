#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keybinding {
    pub key: &'static str,
    pub action: &'static str,
    pub fallback: &'static str,
}

pub const BASIC_KEYBINDINGS: &[Keybinding] = &[
    Keybinding {
        key: "Enter",
        action: "send the current prompt",
        fallback: "type a prompt and press Enter",
    },
    Keybinding {
        key: "Ctrl+C",
        action: "interrupt the current terminal operation",
        fallback: "keyboard interrupt remains handled by the terminal",
    },
    Keybinding {
        key: "copy",
        action: "copy or print a suggested command",
        fallback: "/copy <cmd-id>",
    },
    Keybinding {
        key: "explain",
        action: "explain a suggested command",
        fallback: "/explain <cmd-id>",
    },
    Keybinding {
        key: "discard",
        action: "discard a suggested command",
        fallback: "/discard <cmd-id>",
    },
    Keybinding {
        key: "context",
        action: "show attached context",
        fallback: "/context",
    },
    Keybinding {
        key: "stance",
        action: "show or change stance",
        fallback: "/stance",
    },
];

pub fn render_keybindings() -> String {
    let mut rendered = String::from("Keybindings and fallbacks\n");
    rendered.push_str("Advanced terminal key handling is not active in the line REPL.\n");
    rendered.push_str("Use these slash commands when direct keybindings are unavailable.\n\n");

    for binding in BASIC_KEYBINDINGS {
        rendered.push_str(&format!(
            "- {}: {}; fallback: {}\n",
            binding.key, binding.action, binding.fallback
        ));
    }

    rendered.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_keybinding_fallbacks() {
        let output = render_keybindings();

        assert!(output.contains("/copy <cmd-id>"));
        assert!(output.contains("/explain <cmd-id>"));
        assert!(output.contains("/discard <cmd-id>"));
        assert!(output.contains("/context"));
        assert!(output.contains("Ctrl+C"));
    }
}
