pub mod openai_compatible;
pub mod router;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatResponse {
    Complete(String),
    Stream(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRoute {
    pub role: String,
    pub model: String,
    pub reason: String,
}

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    fn last_model_route(&self) -> Option<ModelRoute> {
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider configuration error: {0}")]
    Configuration(String),
    #[error("provider authentication failed: {0}")]
    Authentication(String),
    #[error("provider network error: {0}")]
    Network(String),
    #[error("provider returned an invalid response: {0}")]
    Response(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingProvider;

    #[async_trait::async_trait]
    impl Provider for FailingProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Err(ProviderError::Authentication("bad key".into()))
        }
    }

    #[tokio::test]
    async fn provider_trait_surfaces_typed_errors() {
        let provider = FailingProvider;
        let error = provider
            .chat(ChatRequest {
                messages: vec![],
                stream: false,
            })
            .await
            .expect_err("provider should fail");

        assert!(matches!(error, ProviderError::Authentication(_)));
    }
}
