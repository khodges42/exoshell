use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::app::CliOptions;
use crate::context::ContextBudget;
use crate::shell::ShellFamily;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub provider: ProviderConfig,
    pub shell: ShellConfig,
    pub transcript: TranscriptConfig,
    pub context: ContextConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub base_url: String,
    pub api_key: String,
    pub api_key_env: String,
    pub model: String,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellConfig {
    pub family: ShellFamily,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptConfig {
    pub directory: PathBuf,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextConfig {
    pub max_characters: Option<usize>,
    pub max_estimated_tokens: Option<usize>,
}

impl ContextConfig {
    pub fn budget(&self) -> ContextBudget {
        ContextBudget {
            max_characters: self.max_characters,
            max_estimated_tokens: self.max_estimated_tokens,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    provider: Option<RawProviderConfig>,
    shell: Option<RawShellConfig>,
    transcript: Option<RawTranscriptConfig>,
    context: Option<RawContextConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct RawProviderConfig {
    base_url: Option<String>,
    api_key_env: Option<String>,
    model: Option<String>,
    request_timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
struct RawShellConfig {
    family: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTranscriptConfig {
    directory: Option<PathBuf>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct RawContextConfig {
    max_characters: Option<usize>,
    max_estimated_tokens: Option<usize>,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let raw = match path {
            Some(path) => RawConfig::from_path(path)?,
            None => RawConfig::from_default_path()?.unwrap_or_default(),
        };

        Self::from_raw(raw)
    }

    fn from_raw(raw: RawConfig) -> Result<Self, ConfigError> {
        let provider = raw.provider.unwrap_or_default();
        let shell = raw.shell.unwrap_or_default();
        let transcript = raw.transcript.unwrap_or_default();
        let context = raw.context.unwrap_or_default();

        let base_url = provider
            .base_url
            .unwrap_or_else(|| "https://api.openai.com/v1".into());
        let api_key_env = provider
            .api_key_env
            .unwrap_or_else(|| "OPENAI_API_KEY".into());
        let api_key = provider_api_key(&base_url, &api_key_env)?;

        let family = shell.family.unwrap_or_else(default_shell_family);
        let family = family
            .parse::<ShellFamily>()
            .map_err(|error| ConfigError::Invalid(error.to_string()))?;

        Ok(Self {
            provider: ProviderConfig {
                base_url,
                api_key,
                api_key_env,
                model: provider.model.unwrap_or_else(|| "gpt-4.1-mini".into()),
                request_timeout_seconds: provider.request_timeout_seconds.unwrap_or(120),
            },
            shell: ShellConfig { family },
            transcript: TranscriptConfig {
                directory: transcript.directory.unwrap_or_else(default_transcript_dir),
                enabled: transcript.enabled.unwrap_or(true),
            },
            context: ContextConfig {
                max_characters: context.max_characters,
                max_estimated_tokens: context.max_estimated_tokens,
            },
        })
    }

    pub fn apply_cli_overrides(&mut self, options: &CliOptions) -> Result<(), ConfigError> {
        if let Some(shell_family) = options.shell_family {
            self.shell.family = shell_family;
        }

        if let Some(transcript_enabled) = options.transcript_enabled {
            self.transcript.enabled = transcript_enabled;
        }

        if let Some(transcript_directory) = &options.transcript_directory {
            self.transcript.directory = transcript_directory.clone();
        }

        Ok(())
    }
}

impl RawConfig {
    fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|error| ConfigError::Read {
            path: path.to_path_buf(),
            error,
        })?;

        toml::from_str(&contents).map_err(ConfigError::Parse)
    }

    fn from_default_path() -> Result<Option<Self>, ConfigError> {
        let path = default_config_path();
        if !path.exists() {
            return Ok(None);
        }

        Self::from_path(&path).map(Some)
    }
}

fn default_shell_family() -> String {
    ShellFamily::default_for_platform().to_string()
}

fn provider_api_key(base_url: &str, api_key_env: &str) -> Result<String, ConfigError> {
    match env::var(api_key_env) {
        Ok(value) => Ok(value),
        Err(_) if is_local_provider_url(base_url) => Ok("exoshell-local-provider".into()),
        Err(_) => Err(ConfigError::MissingApiKey(format!(
            "set {api_key_env} or configure provider.api_key_env"
        ))),
    }
}

fn is_local_provider_url(base_url: &str) -> bool {
    let Some(host) = provider_host(base_url) else {
        return false;
    };

    matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
}

fn provider_host(base_url: &str) -> Option<String> {
    let after_scheme = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url);
    let authority = after_scheme.split('/').next()?.trim();

    if authority.starts_with('[') {
        return authority
            .split(']')
            .next()
            .map(|host| host.trim_start_matches('[').to_ascii_lowercase());
    }

    authority
        .split(':')
        .next()
        .filter(|host| !host.is_empty())
        .map(|host| host.to_ascii_lowercase())
}

fn default_config_path() -> PathBuf {
    if cfg!(windows) {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("exoshell")
            .join("config.toml")
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("exoshell")
            .join("config.toml")
    }
}

fn default_transcript_dir() -> PathBuf {
    if cfg!(windows) {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("exoshell")
            .join("transcripts")
    } else {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("exoshell")
            .join("transcripts")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config at {path}: {error}")]
    Read {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("failed to parse config: {0}")]
    Parse(toml::de::Error),
    #[error("missing provider API key: {0}")]
    MissingApiKey(String),
    #[error("invalid config: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn loads_defaults_from_environment() {
        unsafe {
            env::set_var("EXOSHELL_TEST_KEY", "secret");
        }

        let config = Config::from_raw(RawConfig {
            provider: Some(RawProviderConfig {
                api_key_env: Some("EXOSHELL_TEST_KEY".into()),
                ..RawProviderConfig::default()
            }),
            ..RawConfig::default()
        })
        .expect("config loads");

        assert_eq!(config.provider.api_key, "secret");
        assert_eq!(config.provider.base_url, "https://api.openai.com/v1");
        assert_eq!(config.provider.model, "gpt-4.1-mini");
    }

    #[test]
    fn rejects_unknown_shell_family() {
        unsafe {
            env::set_var("EXOSHELL_TEST_KEY", "secret");
        }

        let error = Config::from_raw(RawConfig {
            provider: Some(RawProviderConfig {
                api_key_env: Some("EXOSHELL_TEST_KEY".into()),
                ..RawProviderConfig::default()
            }),
            shell: Some(RawShellConfig {
                family: Some("cmd".into()),
            }),
            transcript: None,
            context: None,
        })
        .expect_err("shell family should be rejected");

        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn loads_toml_config_file() {
        let mut file = tempfile::NamedTempFile::new().expect("temp config");
        write!(
            file,
            r#"
[provider]
base_url = "http://localhost:11434/v1"
model = "local-model"
request_timeout_seconds = 45

[shell]
family = "posix"

[transcript]
enabled = false

[context]
max_characters = 12000
max_estimated_tokens = 3000
"#
        )
        .expect("write config");

        let config = Config::load(Some(file.path())).expect("config loads");

        assert_eq!(config.provider.base_url, "http://localhost:11434/v1");
        assert_eq!(config.provider.api_key, "exoshell-local-provider");
        assert_eq!(config.provider.model, "local-model");
        assert_eq!(config.provider.request_timeout_seconds, 45);
        assert_eq!(config.shell.family, ShellFamily::Posix);
        assert!(!config.transcript.enabled);
        assert_eq!(config.context.max_characters, Some(12000));
        assert_eq!(config.context.max_estimated_tokens, Some(3000));
        assert_eq!(config.context.budget().max_characters, Some(12000));
    }

    #[test]
    fn context_budget_defaults_to_unlimited() {
        unsafe {
            env::set_var("EXOSHELL_TEST_KEY", "secret");
        }

        let config = Config::from_raw(RawConfig {
            provider: Some(RawProviderConfig {
                api_key_env: Some("EXOSHELL_TEST_KEY".into()),
                ..RawProviderConfig::default()
            }),
            ..RawConfig::default()
        })
        .expect("config loads");

        assert_eq!(config.context.max_characters, None);
        assert_eq!(config.context.max_estimated_tokens, None);
        assert_eq!(config.provider.request_timeout_seconds, 120);
    }

    #[test]
    fn applies_cli_overrides() {
        unsafe {
            env::set_var("EXOSHELL_TEST_KEY", "secret");
        }

        let mut config = Config::from_raw(RawConfig {
            provider: Some(RawProviderConfig {
                api_key_env: Some("EXOSHELL_TEST_KEY".into()),
                ..RawProviderConfig::default()
            }),
            ..RawConfig::default()
        })
        .expect("config loads");

        let tempdir = PathBuf::from("manual-transcripts");
        let options = CliOptions {
            shell_family: Some(ShellFamily::Posix),
            transcript_enabled: Some(false),
            transcript_directory: Some(tempdir.clone()),
            ..CliOptions::default()
        };

        config.apply_cli_overrides(&options).expect("overrides");

        assert_eq!(config.shell.family, ShellFamily::Posix);
        assert!(!config.transcript.enabled);
        assert_eq!(config.transcript.directory, tempdir);
    }

    #[test]
    fn local_provider_urls_do_not_require_api_key_env() {
        let config = Config::from_raw(RawConfig {
            provider: Some(RawProviderConfig {
                base_url: Some("http://127.0.0.1:11434/v1".into()),
                api_key_env: Some("EXOSHELL_MISSING_LOCAL_KEY".into()),
                ..RawProviderConfig::default()
            }),
            ..RawConfig::default()
        })
        .expect("local config should load without key");

        assert_eq!(config.provider.api_key, "exoshell-local-provider");
    }

    #[test]
    fn hosted_provider_urls_require_api_key_env() {
        let error = Config::from_raw(RawConfig {
            provider: Some(RawProviderConfig {
                base_url: Some("https://api.openai.com/v1".into()),
                api_key_env: Some("EXOSHELL_MISSING_HOSTED_KEY".into()),
                ..RawProviderConfig::default()
            }),
            ..RawConfig::default()
        })
        .expect_err("hosted config should require key");

        assert!(matches!(error, ConfigError::MissingApiKey(_)));
    }

    #[test]
    fn detects_local_provider_hosts() {
        assert!(is_local_provider_url("http://localhost:11434/v1"));
        assert!(is_local_provider_url("http://127.0.0.1:11434/v1"));
        assert!(is_local_provider_url("http://[::1]:11434/v1"));
        assert!(!is_local_provider_url("https://api.openai.com/v1"));
    }
}
