use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ProviderError};

#[derive(Debug)]
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleProvider {
    pub fn from_config(config: &Config) -> Result<Self, ProviderError> {
        Self::new(
            config.provider.base_url.clone(),
            config.provider.api_key.clone(),
            config.provider.model.clone(),
        )
    }

    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(ProviderError::Configuration("base URL is empty".into()));
        }

        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(ProviderError::Configuration("API key is empty".into()));
        }

        let model = model.into();
        if model.is_empty() {
            return Err(ProviderError::Configuration("model is empty".into()));
        }

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            model,
        })
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    pub async fn chat_with_model(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, ProviderError> {
        let payload = ChatCompletionRequest {
            model: model.to_string(),
            messages: request.messages,
            stream: request.stream,
        };

        self.send_chat_completion(payload).await
    }

    async fn send_chat_completion(
        &self,
        payload: ChatCompletionRequest,
    ) -> Result<ChatResponse, ProviderError> {
        let response = self
            .client
            .post(self.chat_url())
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|error| ProviderError::Network(error.to_string()))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(ProviderError::Authentication(
                "check provider API key and permissions".into(),
            ));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Response(format!(
                "HTTP {status}: {}",
                body.trim()
            )));
        }

        if payload.stream {
            return read_streaming_response(response).await;
        }

        let body: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|error| ProviderError::Response(error.to_string()))?;

        let content = body
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| ProviderError::Response("missing assistant choice".into()))?;

        Ok(ChatResponse::Complete(content))
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiCompatibleProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let payload = ChatCompletionRequest {
            model: self.model.clone(),
            messages: request.messages,
            stream: request.stream,
        };

        self.send_chat_completion(payload).await
    }
}

async fn read_streaming_response(
    response: reqwest::Response,
) -> Result<ChatResponse, ProviderError> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut chunks = Vec::new();

    while let Some(item) = stream.next().await {
        let bytes = item.map_err(|error| ProviderError::Network(error.to_string()))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };

            let data = data.trim();
            if data == "[DONE]" {
                return Ok(ChatResponse::Stream(chunks));
            }

            let event: ChatCompletionChunk = serde_json::from_str(data)
                .map_err(|error| ProviderError::Response(error.to_string()))?;

            for choice in event.choices {
                if let Some(content) = choice.delta.content {
                    chunks.push(content);
                }
            }
        }
    }

    Ok(ChatResponse::Stream(chunks))
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Debug, Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ChatRole;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn rejects_empty_configuration() {
        let error = OpenAiCompatibleProvider::new("", "key", "model")
            .expect_err("empty base URL should fail");

        assert!(matches!(error, ProviderError::Configuration(_)));
    }

    #[tokio::test]
    async fn sends_openai_compatible_request_and_reads_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
        let address = listener.local_addr().expect("mock address");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut request = vec![0; 4096];
            let bytes = socket.read(&mut request).await.expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);

            assert!(request.contains("POST /chat/completions HTTP/1.1"));
            assert!(request.contains("\"model\":\"test-model\""));
            assert!(request.contains("\"role\":\"user\""));
            assert!(request.contains("\"stream\":false"));

            let body = r#"{"choices":[{"message":{"content":"hello from mock"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );

            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let provider =
            OpenAiCompatibleProvider::new(format!("http://{address}"), "test-key", "test-model")
                .expect("provider");

        let response = provider
            .chat(ChatRequest {
                messages: vec![ChatMessage::new(ChatRole::User, "hello")],
                stream: false,
            })
            .await
            .expect("chat response");

        server.await.expect("server task");
        assert_eq!(response, ChatResponse::Complete("hello from mock".into()));
    }
}
