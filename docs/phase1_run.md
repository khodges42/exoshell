# Phase 1 Local Run

Exoshell Phase 1 is a shell-adjacent model chat REPL. It suggests commands and explanations, but it does not execute commands.

Run locally:

```sh
cargo run
```

Use a config file:

```sh
cargo run -- --config path/to/config.toml
```

Select the command dialect from the CLI:

```sh
cargo run -- --shell powershell
cargo run -- --shell posix
```

Control transcript behavior:

```sh
cargo run -- --no-transcript
cargo run -- --transcript-dir transcripts
```

Example config:

```toml
[provider]
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
model = "gpt-4.1-mini"

[shell]
family = "powershell"

[transcript]
enabled = true
directory = "transcripts"
```

Local OpenAI-compatible servers such as Ollama do not require an API key env var. Example Ollama config:

```toml
[provider]
base_url = "http://localhost:11434/v1"
model = "qwen3-coder-next"

[shell]
family = "powershell"

[transcript]
enabled = true
directory = "transcripts"
```

Supported shell families in this first pass are `powershell` and `posix`.

PowerShell example:

```powershell
$env:OPENAI_API_KEY = "..."
cargo run -- --shell powershell
```

POSIX example:

```sh
export OPENAI_API_KEY="..."
cargo run -- --shell posix
```

Inside the REPL:

- `/exit` quits.
- `/quit` quits.
- `/multi` starts multi-line input. Finish with a single `.` line.

Phase 1 behavior:

- Exoshell suggests commands, but does not execute them.
- Suggested command blocks are labeled for review in terminal output.
- Risky commands should be reviewed manually before use.
- Session transcripts are markdown files when transcripts are enabled.
- Hosted provider API keys are read from environment variables and are not written to transcripts.

Current limitations:

- No PTY integration.
- No hotkey command injection.
- No ambient context collection.
- No repo indexing.
- No autonomous execution.

Quality commands:

```sh
cargo fmt
cargo test
cargo clippy --all-targets --all-features
```

Manual startup smoke test on Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\manual_phase1_startup.ps1
```
