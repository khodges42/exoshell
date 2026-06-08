use std::fmt;
use std::sync::Mutex;

use serde::Deserialize;

use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::{
    ChatMessage, ChatRequest, ChatResponse, ChatRole, ModelRoute, Provider, ProviderError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRouterConfig {
    pub enabled: bool,
    pub model: String,
    pub fallback_role: String,
    pub behavior: String,
    pub roles: Vec<ModelRouterRole>,
}

impl ModelRouterConfig {
    pub fn validate(&self) -> Result<(), ModelRouterConfigError> {
        if self.model.trim().is_empty() {
            return Err(ModelRouterConfigError::Invalid(
                "router model is empty".into(),
            ));
        }
        if self.fallback_role.trim().is_empty() {
            return Err(ModelRouterConfigError::Invalid(
                "router fallback role is empty".into(),
            ));
        }
        if self.behavior.trim().is_empty() {
            return Err(ModelRouterConfigError::Invalid(
                "router behavior is empty".into(),
            ));
        }
        if self.roles.is_empty() {
            return Err(ModelRouterConfigError::Invalid(
                "router requires at least one role".into(),
            ));
        }

        for role in &self.roles {
            if role.name.trim().is_empty() {
                return Err(ModelRouterConfigError::Invalid(
                    "router role name is empty".into(),
                ));
            }
            if role.model.trim().is_empty() {
                return Err(ModelRouterConfigError::Invalid(format!(
                    "router role '{}' has an empty model",
                    role.name
                )));
            }
            if role.description.trim().is_empty() {
                return Err(ModelRouterConfigError::Invalid(format!(
                    "router role '{}' has an empty description",
                    role.name
                )));
            }
        }

        if self.role(&self.fallback_role).is_none() {
            return Err(ModelRouterConfigError::Invalid(format!(
                "router fallback role '{}' is not defined",
                self.fallback_role
            )));
        }

        Ok(())
    }

    pub fn role(&self, name: &str) -> Option<&ModelRouterRole> {
        self.roles.iter().find(|role| role.name == name)
    }

    fn fallback_route(&self, reason: impl Into<String>) -> ModelRoute {
        let role = self
            .role(&self.fallback_role)
            .expect("validated router fallback role exists");
        ModelRoute {
            role: role.name.clone(),
            model: role.model.clone(),
            reason: reason.into(),
        }
    }
}

impl Default for ModelRouterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "qwen2.5-coder:7b".into(),
            fallback_role: "coding".into(),
            behavior: default_router_behavior().into(),
            roles: default_router_roles(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRouterRole {
    pub name: String,
    pub model: String,
    pub description: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelRouterConfigError {
    #[error("invalid model router config: {0}")]
    Invalid(String),
}

pub struct ModelRouterProvider {
    provider: OpenAiCompatibleProvider,
    config: ModelRouterConfig,
    last_route: Mutex<Option<ModelRoute>>,
}

impl ModelRouterProvider {
    pub fn new(
        provider: OpenAiCompatibleProvider,
        config: ModelRouterConfig,
    ) -> Result<Self, ProviderError> {
        config
            .validate()
            .map_err(|error| ProviderError::Configuration(error.to_string()))?;
        Ok(Self {
            provider,
            config,
            last_route: Mutex::new(None),
        })
    }

    async fn route(&self, request: &ChatRequest) -> ModelRoute {
        let router_request = ChatRequest {
            messages: vec![
                ChatMessage::new(ChatRole::System, self.router_system_prompt()),
                ChatMessage::new(ChatRole::User, router_user_prompt(request)),
            ],
            stream: false,
        };

        let response = self
            .provider
            .chat_with_model(router_request, &self.config.model)
            .await;

        match response {
            Ok(ChatResponse::Complete(content)) => parse_router_response(&content, &self.config)
                .unwrap_or_else(|| {
                    self.config
                        .fallback_route(format!("router returned unrecognized role: {content}"))
                }),
            Ok(ChatResponse::Stream(chunks)) => {
                let content = chunks.concat();
                parse_router_response(&content, &self.config).unwrap_or_else(|| {
                    self.config
                        .fallback_route(format!("router returned unrecognized role: {content}"))
                })
            }
            Err(error) => self
                .config
                .fallback_route(format!("router failed: {error}")),
        }
    }

    fn router_system_prompt(&self) -> String {
        let mut prompt = String::new();
        prompt.push_str("You are Exoshell's model router. Choose exactly one role for the next assistant response.\n");
        prompt.push_str(&self.config.behavior);
        prompt.push_str("\n\nAvailable roles:\n");
        for role in &self.config.roles {
            prompt.push_str(&format!(
                "- {}: {} (model: {})\n",
                role.name, role.description, role.model
            ));
        }
        prompt.push_str(
            "\nRespond as compact JSON only: {\"role\":\"<role>\",\"reason\":\"<short reason>\"}",
        );
        prompt
    }
}

#[async_trait::async_trait]
impl Provider for ModelRouterProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let route = self.route(&request).await;
        {
            let mut last_route = self
                .last_route
                .lock()
                .expect("model route lock should not be poisoned");
            *last_route = Some(route.clone());
        }

        self.provider.chat_with_model(request, &route.model).await
    }

    fn last_model_route(&self) -> Option<ModelRoute> {
        self.last_route
            .lock()
            .expect("model route lock should not be poisoned")
            .clone()
    }
}

fn router_user_prompt(request: &ChatRequest) -> String {
    let mut rendered = String::new();
    rendered.push_str("Route this request using the current prompt payload.\n\n");
    for message in &request.messages {
        rendered.push_str(&format!("{:?}:\n{}\n\n", message.role, message.content));
    }
    rendered
}

fn parse_router_response(content: &str, config: &ModelRouterConfig) -> Option<ModelRoute> {
    if let Ok(response) = serde_json::from_str::<RouterResponse>(content.trim())
        && let Some(role) = config.role(response.role.trim())
    {
        return Some(ModelRoute {
            role: role.name.clone(),
            model: role.model.clone(),
            reason: response
                .reason
                .unwrap_or_else(|| "router selected role".into()),
        });
    }

    let lowered = content.to_ascii_lowercase();
    config.roles.iter().find_map(|role| {
        if lowered.contains(&role.name.to_ascii_lowercase()) {
            Some(ModelRoute {
                role: role.name.clone(),
                model: role.model.clone(),
                reason: "router selected role from text response".into(),
            })
        } else {
            None
        }
    })
}

#[derive(Debug, Deserialize)]
struct RouterResponse {
    role: String,
    reason: Option<String>,
}

fn default_router_behavior() -> &'static str {
    "Prefer the cheapest and fastest role that is likely to answer well. Use instant for simple routing, short shell questions, quick lookups, and low-risk responses. Use coding for source changes, debugging, tests, command construction, and repository work. Use heavy for complex architecture, multi-step reasoning, risky operational analysis, or large-context synthesis. Use conversational for general explanation, planning, and non-code discussion."
}

fn default_router_roles() -> Vec<ModelRouterRole> {
    vec![
        ModelRouterRole {
            name: "instant".into(),
            model: "qwen2.5-coder:7b".into(),
            description: "fast responses for simple prompts and low-latency checks".into(),
        },
        ModelRouterRole {
            name: "coding".into(),
            model: "coder-g4-26b".into(),
            description: "code edits, debugging, tests, and shell command construction".into(),
        },
        ModelRouterRole {
            name: "heavy".into(),
            model: "coder-g4-26b".into(),
            description: "complex reasoning, architecture, and high-context technical work".into(),
        },
        ModelRouterRole {
            name: "conversational".into(),
            model: "qwen2.5-coder:7b".into(),
            description: "general discussion, planning, and explanations".into(),
        },
    ]
}

impl fmt::Debug for ModelRouterProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelRouterProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_router_roles_match_expected_models() {
        let config = ModelRouterConfig::default();

        assert_eq!(
            config.role("instant").expect("instant").model,
            "qwen2.5-coder:7b"
        );
        assert_eq!(
            config.role("conversational").expect("conversational").model,
            "qwen2.5-coder:7b"
        );
        assert_eq!(config.role("coding").expect("coding").model, "coder-g4-26b");
        assert_eq!(config.role("heavy").expect("heavy").model, "coder-g4-26b");
        config.validate().expect("default config validates");
    }

    #[test]
    fn parses_json_router_response() {
        let config = ModelRouterConfig::default();
        let route = parse_router_response(
            r#"{"role":"heavy","reason":"large context architecture"}"#,
            &config,
        )
        .expect("route");

        assert_eq!(route.role, "heavy");
        assert_eq!(route.model, "coder-g4-26b");
        assert_eq!(route.reason, "large context architecture");
    }

    #[test]
    fn rejects_missing_fallback_role() {
        let config = ModelRouterConfig {
            fallback_role: "missing".into(),
            ..ModelRouterConfig::default()
        };

        assert!(config.validate().is_err());
    }
}
