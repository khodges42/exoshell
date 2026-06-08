# Phase 2 Interaction Model

Phase 2 makes Exoshell behave more like an operational overlay than a generic terminal chat client.

The shell remains primary. Exoshell suggests commands, explains tradeoffs, preserves explicit context, and records transcript events. It does not execute commands.

## Core Objects

User input is the operator's prompt in the REPL or a multi-line prompt collected with `/multi`.

Attached context is explicit, session-scoped material selected by the operator. Context comes from the Phase 1.5 context engine: notes, files, pasted command output, piped stdin, and directory summaries. Disabled context remains visible in the session but is not sent to the model.

Stance is a compact behavior fragment added to the system prompt. Supported stances are `operator`, `audit`, `teach`, and `quiet`.

Model response is provider output recorded as assistant transcript text. Exoshell may parse fenced shell blocks from the response as command suggestions.

Command suggestion is a shell fenced code block such as `powershell`, `pwsh`, `sh`, `bash`, `zsh`, or `posix`. Suggestions receive per-response IDs such as `cmd-001`. Exoshell can print, explain, or discard a suggestion by ID. These actions do not execute the command.

Transcript entry is a markdown record of user prompts, assistant responses, context events, budget warnings, stance changes, command suggestions, and command actions.

## Prompt Assembly

Prompt assembly is deterministic:

1. Phase 2 system prompt with the selected shell family and stance.
2. Conversation history before the current user prompt.
3. Attached context rendered by the Phase 1.5 prompt context renderer.
4. Current user prompt.

Context rendering preserves context IDs, types, titles, provenance, priority, and truncation metadata when present.

The context budget check uses the Phase 1.5 context budget settings. `/context stats` and `/panel` also show an approximate full prompt estimate covering the system prompt, conversation history, and attached context.

## Stances

`operator` favors concise next steps, commands, and operational diagnosis.

`audit` prioritizes security, correctness, reliability, and failure modes. It asks the model to separate evidence from inference.

`teach` explains commands and concepts more fully while staying usable in a terminal.

`quiet` minimizes prose and focuses on direct output. It does not suppress safety warnings.

Configure the default stance:

```toml
[interaction]
stance = "audit"
```

Override it from the CLI:

```sh
cargo run -- --stance teach
```

Switch it during a session:

```text
/stance audit
```

## Command Suggestions

The model is instructed to put suggested commands in shell fenced code blocks. Exoshell parses those blocks and displays action IDs.

Example response fragment:

~~~text
Inspect tracked changes first:
```sh
git status --short
```
~~~

Available actions:

```text
/copy cmd-001
/explain cmd-001
/discard cmd-001
```

`/copy` prints the command when clipboard support is unavailable. It does not run the command.

`/explain` describes the shell family, original command text, model note when available, and detected risk warning.

`/discard` marks the suggestion as discarded in session state and records the action in the transcript.

## Risk Detection

Phase 2 includes a simple non-blocking detector for obvious risky commands. The detector is policy-driven: Exoshell ships with conservative defaults, and users can add or replace rules at runtime through config.

Default rules flag patterns such as recursive forced deletion, disk formatting, recursive permission changes, downloaded content piped to an interpreter, credential exposure, and package removal.

Example custom rule:

```toml
[commands.risk]
include_defaults = true

[[commands.risk.rules]]
match_all = ["kubectl delete", "--all"]
reason = "cluster-wide deletion"
shell = "posix"
```

False positives are acceptable. A warning means "review this carefully," not "this command is definitely harmful." Lack of a warning does not mean a command is safe.

## Session Controls

Useful commands:

```text
/panel
/context
/context stats
/help
/help context
/help stance
/help commands
```

`/panel` renders stance, shell family, provider/model, transcript state, context entries, and prompt estimates without requiring a TUI.

## Non-Goals

Phase 2 does not make Exoshell an autonomous executor.

Phase 2 does not add hidden memory. Context remains explicit and session-scoped.

Phase 2 does not perform full repository indexing. Project awareness belongs to Phase 3.

Phase 2 does not claim command suggestions are safe. It provides review language and simple detection only.

## Known Limitations

Clipboard integration is not implemented. `/copy` prints the command.

Command parsing is intentionally simple and based on fenced blocks.

Risk detection is heuristic and incomplete.

Advanced TUI keybindings and config profiles remain planned work.
