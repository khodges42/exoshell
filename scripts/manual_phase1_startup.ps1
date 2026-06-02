$ErrorActionPreference = "Stop"

$env:OPENAI_API_KEY = "manual-startup-placeholder"
$configPath = Join-Path $env:TEMP "exoshell-manual-startup.toml"
@"
[provider]
api_key_env = "OPENAI_API_KEY"
model = "manual-startup-model"

[shell]
family = "powershell"

[transcript]
enabled = false
"@ | Set-Content -Path $configPath -Encoding utf8

"/exit" | cargo run -- --config $configPath
