# Known Bugs and Limitations

## Phase 2 command controls

Clipboard integration is not implemented. `/copy <cmd-id>` prints the command instead of writing to the system clipboard.

Command suggestion parsing is based on shell fenced code blocks and does not understand arbitrary prose or malformed markdown.

Destructive command detection is heuristic. It warns on obvious high-risk patterns but cannot prove that an unflagged command is safe.

Advanced TUI keybindings, config profiles, one-shot prompt mode, and user-marked discoveries remain planned Phase 2 work.

## Local verification environment

The current local WSL toolchain reports Cargo 1.75.0 without `rustfmt` or `rustup`. This cannot run the repository's Rust 2024 project locally without installing a newer toolchain.
