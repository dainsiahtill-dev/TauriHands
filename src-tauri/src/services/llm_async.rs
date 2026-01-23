use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

use crate::services::llm::{LlmProfile, LlmResponseFormat};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncLlmRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
    pub response_format: Option<LlmResponseFormat>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncLlmResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub struct AsyncLlmService {
    client: Client,
    profile: LlmProfile,
}

impl AsyncLlmService {
    pub fn new(profile: LlmProfile) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, profile }
    }

    pub async fn request_completion(&self, messages: &[ChatMessage]) -> Result<AsyncLlmResponse, String> {
        let request = AsyncLlmRequest {
            model: self.profile.model.clone(),
            messages: messages.to_vec(),
            temperature: 0.7,
            max_tokens: 2048,
            stream: false,
            response_format: None,
        };

        let url = format!("{}/chat/completions", self.profile.baseUrl.trim_end_matches('/'));
        
        let response = timeout(
            Duration::from_secs(60),
            self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.profile.apiKey))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

        let response = response
            .map_err(|e| format!("HTTP error: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Status error: {}", e))?;

        response
            .json::<AsyncLlmResponse>()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    pub async fn request_completion_stream(
        &self,
        messages: &[ChatMessage],
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>, String> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        
        let request = AsyncLlmRequest {
            model: self.profile.model.clone(),
            messages: messages.to_vec(),
            temperature: 0.7,
            max_tokens: 2048,
            stream: true,
            response_format: None,
        };

        let url = format!("{}/chat/completions", self.profile.baseUrl.trim_end_matches('/'));
        let client = self.client.clone();
        let api_key = self.profile.apiKey.clone();

        tokio::spawn(async move {
            match timeout(
                Duration::from_secs(120),
                client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
            )
            .await
            {
                Ok(Ok(response)) => {
                    match response.error_for_status() {
                        Ok(response) => {
                            if let Ok(bytes) = response.bytes().await {
                                let chunk_str = String::from_utf8_lossy(&bytes);
                                for line in chunk_str.lines() {
                                    if line.starts_with("data: ") && line.len() > 6 {
                                        let data = &line[6..];
                                        if data.trim() == "[DONE]" {
                                            break;
                                        }
                                        if let Ok(_) = tx.send(data.to_string()) {
                                            // Successfully sent chunk
                                        } else {
                                            break; // Channel closed
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(format!("Stream error: {}", e));
                        }
                    }
                }
                Ok(Err(e)) => {
                    let _ = tx.send(format!("Request failed: {}", e));
                }
                Err(_) => {
                    let _ = tx.send("Request timeout".to_string());
                }
            }
        });

        Ok(rx)
    }

    pub async fn fetch_models(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/models", self.profile.baseUrl.trim_end_matches('/'));
        
        let response = timeout(
            Duration::from_secs(30),
            self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.profile.apiKey))
                .send()
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

        let response = response
            .map_err(|e| format!("HTTP error: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Status error: {}", e))?;

        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelData>,
        }

        #[derive(Deserialize)]
        struct ModelData {
            id: String,
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(models_response.data.into_iter().map(|m| m.id).collect())
    }
}

#[async_trait]
pub trait AsyncLlmProvider {
    async fn complete(&self, messages: &[ChatMessage]) -> Result<String, String>;
    async fn complete_stream(&self, messages: &[ChatMessage]) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>, String>;
    async fn list_models(&self) -> Result<Vec<String>, String>;
}

#[async_trait]
impl AsyncLlmProvider for AsyncLlmService {
    async fn complete(&self, messages: &[ChatMessage]) -> Result<String, String> {
        let response = self.request_completion(messages).await?;
        Ok(response.choices.first()
            .ok_or("No choices in response")?
            .message
            .content
            .clone())
    }

    async fn complete_stream(&self, messages: &[ChatMessage]) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>, String> {
        self.request_completion_stream(messages).await
    }

    async fn list_models(&self) -> Result<Vec<String>, String> {
        self.fetch_models().await
    }
}
