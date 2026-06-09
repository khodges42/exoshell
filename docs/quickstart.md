# Quickstart

This guide gets Exoshell running and shows the main Phase 2 workflow.

Exoshell suggests commands and explains system work. It does not execute commands.

## Prerequisites

Install a current Rust toolchain with `rustup`.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile default --default-toolchain stable
. "$HOME/.cargo/env"
```

Check the toolchain:

```sh
cargo --version
rustc --version
rustfmt --version
```

On Ubuntu or WSL, provider dependencies may also need:

```sh
sudo apt update
sudo apt install pkg-config libssl-dev
```

## Configure a Provider

For OpenAI-compatible hosted providers, set an API key:

```sh
export OPENAI_API_KEY="..."
```

For a local OpenAI-compatible provider, create a config file:

```toml
[provider]
base_url = "http://localhost:11434/v1"
model = "local-model"
request_timeout_seconds = 120

[shell]
family = "posix"

[interaction]
stance = "operator"
```

Local provider URLs such as `localhost` and `127.0.0.1` do not require an API key.

## Configure Command Risk Rules

Exoshell ships with conservative default rules for obvious risky command suggestions. You can add your own rules at runtime:

```toml
[commands.risk]
include_defaults = true

[[commands.risk.rules]]
match_all = ["kubectl delete", "--all"]
reason = "cluster-wide deletion"
shell = "posix"

[[commands.risk.rules]]
match_all = ["terraform apply"]
reason = "infrastructure mutation"
shell = "posix"
```

`match_all` is a list of case-insensitive substrings that must all appear in the suggested command. `shell` is optional; use `posix` or `powershell`.

To replace the built-in defaults entirely:

```toml
[commands.risk]
include_defaults = false
```

## Configure Model Routing

Exoshell can route each prompt through a fast router model before selecting the model that should answer.

Enable the default router:

```toml
[router]
enabled = true
model = "qwen2.5-coder:7b"
fallback_role = "coding"
```

Default roles:

```text
instant          qwen2.5-coder:7b
coding           coder-g4-26b
heavy            coder-g4-26b
conversational   qwen2.5-coder:7b
```

Override role models or behavior:

```toml
[router]
enabled = true
model = "qwen2.5-coder:7b"
fallback_role = "coding"
behavior = "Prefer instant for short shell questions. Use heavy only for architecture or high-context analysis."

[[router.roles]]
name = "instant"
model = "qwen2.5-coder:7b"
description = "fast responses for simple prompts"

[[router.roles]]
name = "coding"
model = "coder-g4-26b"
description = "code edits, debugging, tests, and shell command construction"

[[router.roles]]
name = "heavy"
model = "coder-g4-26b"
description = "complex reasoning and architecture"

[[router.roles]]
name = "conversational"
model = "qwen2.5-coder:7b"
description = "general discussion and explanations"
```

For Ollama model setup examples, see [khodges42/modelfiles](https://github.com/khodges42/modelfiles).

## Start Exoshell

Run with defaults:

```sh
cargo run
```

Run with a config file:

```sh
cargo run -- --config path/to/config.toml
```

Select a shell family:

```sh
cargo run -- --shell posix
cargo run -- --shell powershell
```

Select an operating stance:

```sh
cargo run -- --stance audit
```

Override detected project root:

```sh
cargo run -- --project-root path/to/repo
```

Or configure it:

```toml
[project]
root = "path/to/repo"
```

## First Session

At the prompt:

```text
exo> /help
```

Attach a note as explicit context:

```text
exo> /add-note this repo is a Rust CLI called Exoshell
```

Inspect context:

```text
exo> /context
exo> /context stats
```

Inspect detected Git project state:

```text
exo> /project
```

Ask a question:

```text
exo> What should I inspect before changing command parsing?
```

Exoshell may return suggested commands in fenced shell blocks. Suggested commands are reviewable text. You decide whether to copy and run them in your shell.

## Context Tour

Context is explicit and session-scoped. Exoshell only sends enabled context to the model.

Add a file:

```text
exo> /add-file Cargo.toml
```

Add a shallow directory summary:

```text
exo> /add-dir src
```

Paste command output without Exoshell running the command:

```text
exo> /add-output
paste command output; finish with a single '.' line
... test result: ok
... .
```

Inspect one entry:

```text
exo> /context show ctx-001
```

Control inclusion:

```text
exo> /context disable ctx-001
exo> /context enable ctx-001
```

Control pruning preference:

```text
exo> /context pin ctx-001
exo> /context priority ctx-001 high
```

Remove an entry:

```text
exo> /context remove ctx-001
```

## Stance Tour

Stances change the compact behavior fragment in the prompt.

Show the current stance:

```text
exo> /stance
```

Switch stance:

```text
exo> /stance operator
exo> /stance audit
exo> /stance teach
exo> /stance quiet
```

Use `operator` for concise next steps, `audit` for risk review, `teach` for fuller explanations, and `quiet` for minimal prose.

## Command Suggestion Tour

When a model response includes shell fenced blocks, Exoshell assigns command IDs such as `cmd-001`.

Print a suggested command:

```text
exo> /copy cmd-001
```

Clipboard support is not implemented yet, so `/copy` prints the command. It does not execute it.

Explain a suggestion:

```text
exo> /explain cmd-001
```

Discard a suggestion:

```text
exo> /discard cmd-001
```

Risk warnings are heuristic. Treat a warning as a prompt for careful review. Lack of a warning does not prove a command is safe.

## Session Panel

Show the current operating state:

```text
exo> /panel
```

The panel includes stance, shell family, provider/model, transcript state, detected Git project, context entries, and prompt estimates.

## Keybinding Fallbacks

The current REPL is line-oriented. Use `/keys` to show the available key actions and their slash-command fallbacks:

```text
exo> /keys
```

Copy, explain, discard, context, and stance actions degrade to explicit commands such as `/copy <cmd-id>`, `/explain <cmd-id>`, `/discard <cmd-id>`, `/context`, and `/stance`.

## Multi-Line Prompts

Use `/multi` for longer prompts:

```text
exo> /multi
multi-line input; finish with a single '.' line
... Review this plan:
... 1. Add parser tests.
... 2. Refactor command rendering.
... .
```

## Piped Input

Pipe text into Exoshell as explicit context:

```sh
printf 'build failed in openssl-sys\n' | cargo run
```

Exoshell records piped content as user-provided context. It does not claim to know the upstream command unless you provide that separately.

## Quality Checks

Run:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features
```

If `cargo test` fails on Ubuntu or WSL with an OpenSSL or `pkg-config` error, install:

```sh
sudo apt install pkg-config libssl-dev
```

## Exit

Quit the REPL:

```text
exo> /exit
```

If transcripts are enabled, Exoshell writes a markdown transcript at shutdown.
