# Context Engine

The context engine stores explicit, user-controlled context for a session.

Context is not hidden memory. Entries are added through commands, startup flags, or piped stdin. Enabled entries are rendered into provider requests before the current user prompt. Disabled entries remain visible in the session but are not sent to the provider.

## Entry Model

Each context entry has:

* stable ID such as `ctx-001`
* type
* title
* enabled flag
* pinned flag
* priority
* creation timestamp
* provenance
* content
* character count and approximate token estimate

Token estimates are deterministic and approximate. The current estimate is character count divided by four, rounded up.

## Providers

Built-in providers:

* `manual`: user-provided note text
* `file`: UTF-8 text file loading
* `command_output`: user-provided stdout/stderr
* `stdin`: piped stdin
* `directory_summary`: bounded directory listing without reading file contents

Providers return `ContextEntry` values and do not talk to model providers or terminal UI code.

## Commands

Inspection and mutation:

```text
/context
/context show <id>
/context remove <id>
/context enable <id>
/context disable <id>
/context pin <id>
/context unpin <id>
/context priority <id> <low|normal|high|critical>
/context stats
```

Adding context:

```text
/add-note <text>
/add-file <path>
/add-dir <path>
/add-output
```

`/add-output` opens a multi-line paste flow and records the text as user-provided command output. Exoshell does not execute the command.

Startup flags:

```text
--context-note <text>
--context-file <path>
```

Piped stdin is imported as explicit context with `stdin` provenance. Exoshell does not claim to know the upstream command unless it was explicitly provided by a caller.

## Budgets

Optional config:

```toml
[context]
max_characters = 12000
max_estimated_tokens = 3000
```

If these fields are absent, there is no explicit context budget limit.

Before a provider request, Exoshell checks enabled context against the configured budget. If the budget is exceeded, the request is stopped with a warning. Context is not silently dropped.

Pruning is deterministic and non-mutating. Low-priority unpinned entries are selected for removal first. Pinned entries and critical-priority entries are preserved as long as possible.

## Transcripts

Context lifecycle events are recorded as metadata:

* add
* remove
* enable
* disable
* pin
* unpin
* priority
* budget warning

Transcript events do not include full context payloads by default.

## Serialization

Serialized context uses version `1`.

By default, serialized context excludes content payloads. Callers must explicitly request content inclusion. This keeps saved context inspectable without assuming it is safe to persist arbitrary file contents, command output, or pasted logs.

## Redaction

Exoshell cannot guarantee automatic secret detection.

Provider metadata can mark fields as sensitive. Transcript rendering also redacts obvious secret-shaped metadata keys such as `api_key`, `token`, `secret`, and `password`.

This is a guardrail, not a security boundary. Users should inspect context before sending or saving it.
