# Exoshell Phases

This roadmap turns the project vision into concrete build phases. The phases are intentionally ordered from the smallest useful shell-adjacent assistant toward the broader cognitive shell described in `README.md` and `docs/DESIGN.md`.

## Phase 1: Shell-Adjacent Model Chat

Goal: ship a usable terminal program that lets a user talk to a model from PowerShell, bash, zsh, and similar shells without replacing the shell.

Core features:

- CLI entry point, likely `exoshell`, that runs as a normal terminal application.
- Provider configuration for at least one OpenAI-compatible backend.
- Local-first-compatible model adapter interface, even if the first implementation targets one backend.
- Interactive chat loop with streaming responses.
- Basic command suggestion formatting that distinguishes prose, shell commands, and warnings.
- Explicit copy/paste-oriented workflow: Exoshell suggests commands, the user remains responsible for execution.
- Cross-platform terminal behavior on Windows, macOS, and Linux.
- Shell-family awareness for PowerShell versus POSIX-like shells.
- Basic session transcript saved as markdown.
- Minimal config file for provider, model, default shell family, and notebook location.
- Clear error handling for missing API keys, provider failures, and interrupted requests.

User experience:
- User should be able to run "exo this is a prompt" and get a response, the program exiting
- User should be able to run exo (without a prompt) and have an open session, or "exo this is a prompt --stay" (or a more appropriate flag) to keep the shell/conversation open
- User should be able to pipe (bash) or powershell equivalent into exo and have it treated as a prompt, with the additional context of the command run that sent the pipe, cwd, etc.
    - For example, "ls | exo" should send a prompt similar to "{"command": "ls", "stdout": "foo bar baz", "cwd": "/foo/bar"}" and any relevant system prompts
Non-goals:

- PTY control of the user's shell.
- Hotkeys that inject commands into the shell.
- Ambient context collection.
- Tree-sitter repo indexing.
- Long-term memory.
- Stances and personalities beyond a fixed default behavior.
- Autonomous command execution.

Exit criteria:

- A user can install/run Exoshell, configure a model backend, ask for help from a terminal, receive command suggestions appropriate to PowerShell or bash/zsh, and save the session transcript.
- The implementation has a small automated test suite for provider abstraction, shell-family prompt behavior, config loading, and transcript writing.

## Phase 2: Context, Modes, and Operator Controls

Goal: make Exoshell feel like an operational overlay instead of a generic chat client.

Core features:

- Explicit context commands for adding files, command output, directory summaries, and pasted logs.
- Session-scoped context panel or context summary visible in the TUI.
- Stances such as `operator`, `audit`, `teach`, and `quiet`.
- Hotkeys for accepting, copying, explaining, and discarding suggested commands.
- Safer command suggestion UI with destructive-command detection and confirmation language.
- Context budget management with visible token or character estimates.
- Shell-specific command rendering and explanation for PowerShell and POSIX-like shells.
- Notebook improvements: title, timestamps, model metadata, commands suggested, and user-marked discoveries.
- Basic configuration profiles for different providers and shells.

Non-goals:

- Passive screen watching.
- Full PTY embedding.
- Repository-wide semantic indexing.
- Long-term memory outside user-controlled markdown/log files.

Exit criteria:

- A user can deliberately attach context, switch stance, inspect what context is being sent, and act on suggestions through predictable terminal controls.

## Phase 3: Repo Awareness and Operational Notebooks

Goal: give Exoshell useful project awareness while keeping memory inspectable and scoped.

Core features:

- Repo detection and project-root selection.
- File tree summaries with ignore rules.
- Structural parsing for common languages using tree-sitter where practical.
- Commands to add source files, symbols, recent git changes, and test output to context.
- Markdown notebooks scoped by global, repo, task, and session.
- Searchable operational notes and discoveries.
- Runbook generation from session notes.
- Diff-oriented patch suggestions that require user review.
- Better uncertainty reporting through explicit signal strength.

Non-goals:

- Hidden long-term behavioral profiling.
- Blind patch application.
- Autonomous multi-file rewrites.

Exit criteria:

- A user can open a repo, ask informed questions about local code and recent command output, preserve useful notes, and generate reviewable runbooks or patch suggestions.

## Phase 4: Follow Mode and Shell Integration

Goal: let Exoshell observe explicit shell activity and provide contextual help without becoming a replacement shell.

Core features:

- PTY or shell-hook integration strategy for supported environments.
- Follow mode that can ingest selected command history, stdout/stderr snippets, current directory, and exit status.
- User-visible controls for what is observed and retained.
- Hotkey workflows for sending command output to Exoshell.
- Optional paste-to-shell and paste-and-run flows with clear confirmation boundaries.
- Command outcome interpretation and next-step suggestions.
- Per-shell integration docs for PowerShell, bash, zsh, and fish.

Non-goals:

- Covert monitoring.
- Unreviewed execution.
- Replacing the user's shell configuration.

Exit criteria:

- A user can work in their normal shell while Exoshell follows explicitly permitted context and offers useful, nonintrusive operational guidance.

## Phase 5: Tuning, Extensibility, and Local-First Depth

Goal: make Exoshell configurable, hackable, and effective with weaker local models.

Core features:

- Robust local model support through OpenAI-compatible local servers and/or dedicated adapters.
- Prompt and stance customization.
- User-editable command policies and safety rules.
- Extension points for tools, context providers, and notebook processors.
- Model routing by task type.
- Context compression and retrieval tuned for smaller models.
- Export/import of user configuration and notebooks.
- Advanced visual themes that remain restrained and terminal-native.

Non-goals:

- Cloud lock-in.
- Manager analytics.
- Personality behavior that compromises operational clarity.

Exit criteria:

- A technical user can tune Exoshell to their environment, run local-first workflows, and extend context/tool behavior without modifying core code.

## Phase 6: Mature Cognitive Shell Overlay

Goal: converge on the broader vision: a calm, inspectable, practitioner-oriented cognitive shell environment.

Core features:

- Mature cockpit-style TUI with dense but readable operational instrumentation.
- Stable shell integrations across major platforms.
- High-quality repo, session, task, and notebook workflows.
- Reviewable command, patch, and runbook pipelines.
- Strong safety posture around destructive commands and hidden state.
- Comprehensive docs for installation, configuration, shell integration, model backends, stances, notebooks, and local-first operation.
- Packaging and release automation.

Exit criteria:

- Exoshell is a dependable daily-driver assistant for terminal-native practitioners who want AI augmentation while preserving control and understanding.
