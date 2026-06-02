use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::app::CliOptions;
use crate::shell::ShellFamily;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub provider: ProviderConfig,
    pub shell: ShellConfig,
    pub transcript: TranscriptConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub base_url: String,
    pub api_key: String,
    pub api_key_env: String,
    pub model: String,
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

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    provider: Option<RawProviderConfig>,
    shell: Option<RawShellConfig>,
    transcript: Option<RawTranscriptConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct RawProviderConfig {
    base_url: Option<String>,
    api_key_env: Option<String>,
    model: Option<String>,
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

        let api_key_env = provider
            .api_key_env
            .unwrap_or_else(|| "OPENAI_API_KEY".into());
        let api_key = env::var(&api_key_env).map_err(|_| {
            ConfigError::MissingApiKey(format!(
                "set {api_key_env} or configure provider.api_key_env"
            ))
        })?;

        let family = shell.family.unwrap_or_else(default_shell_family);
        let family = family
            .parse::<ShellFamily>()
            .map_err(|error| ConfigError::Invalid(error.to_string()))?;

        Ok(Self {
            provider: ProviderConfig {
                base_url: provider
                    .base_url
                    .unwrap_or_else(|| "https://api.openai.com/v1".into()),
                api_key,
                api_key_env,
                model: provider.model.unwrap_or_else(|| "gpt-4.1-mini".into()),
            },
            shell: ShellConfig { family },
            transcript: TranscriptConfig {
                directory: transcript.directory.unwrap_or_else(default_transcript_dir),
                enabled: transcript.enabled.unwrap_or(true),
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
        })
        .expect_err("shell family should be rejected");

        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn loads_toml_config_file() {
        unsafe {
            env::set_var("EXOSHELL_TEST_KEY", "secret");
        }

        let mut file = tempfile::NamedTempFile::new().expect("temp config");
        write!(
            file,
            r#"
[provider]
base_url = "http://localhost:11434/v1"
api_key_env = "EXOSHELL_TEST_KEY"
model = "local-model"

[shell]
family = "posix"

[transcript]
enabled = false
"#
        )
        .expect("write config");

        let config = Config::load(Some(file.path())).expect("config loads");

        assert_eq!(config.provider.base_url, "http://localhost:11434/v1");
        assert_eq!(config.provider.model, "local-model");
        assert_eq!(config.shell.family, ShellFamily::Posix);
        assert!(!config.transcript.enabled);
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
}
