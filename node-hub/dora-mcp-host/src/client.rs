use tokio::sync::mpsc;

use eyre::{eyre, Result};
use futures::channel::oneshot;
use reqwest::Client as HttpClient;
use salvo::async_trait;

use crate::config::{DeepseekConfig, DoraConfig, GeminiConfig, MoonshotConfig, OpenaiConfig};
use crate::models::{ChatCompletionRequest, ChatCompletionResponse};
use crate::ServerEvent;

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse>;
}

#[derive(Debug)]
pub struct GeminiClient {
    api_key: String,
    api_url: String,
    client: HttpClient,
}

impl GeminiClient {
    pub fn new(config: &GeminiConfig) -> Self {
        let client = if config.proxy {
            HttpClient::new()
        } else {
            HttpClient::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| HttpClient::new())
        };

        Self {
            api_key: config.api_key.clone(),
            api_url: config.api_url.clone(),
            client,
        }
    }
}

#[async_trait]
impl ChatClient for GeminiClient {
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let response = self
            .client
            .post(&self.api_url)
            .header("X-goog-api-key", self.api_key.clone())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(eyre!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: ChatCompletionResponse = serde_json::from_str(&text_data)
            .map_err(eyre::Report::from)
            .unwrap();
        Ok(completion)
    }
}

#[derive(Debug)]
pub struct DeepseekClient {
    api_key: String,
    api_url: String,
    client: HttpClient,
}

impl DeepseekClient {
    pub fn new(config: &DeepseekConfig) -> Self {
        let client = if config.proxy {
            HttpClient::new()
        } else {
            HttpClient::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| HttpClient::new())
        };

        Self {
            api_key: config.api_key.clone(),
            api_url: config.api_url.clone(),
            client,
        }
    }
}

#[async_trait]
impl ChatClient for DeepseekClient {
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let response = self
            .client
            .post(&format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(eyre!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: ChatCompletionResponse =
            serde_json::from_str(&text_data).map_err(eyre::Report::from)?;
        Ok(completion)
    }
}

#[derive(Debug)]
pub struct OpenaiClient {
    api_key: String,
    api_url: String,
    client: HttpClient,
}

impl OpenaiClient {
    pub fn new(config: &OpenaiConfig) -> Self {
        let client = if config.proxy {
            HttpClient::new()
        } else {
            HttpClient::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| HttpClient::new())
        };

        Self {
            api_key: config.api_key.clone(),
            api_url: config.api_url.clone(),
            client,
        }
    }
}

#[async_trait]
impl ChatClient for OpenaiClient {
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let response = self
            .client
            .post(&format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(eyre!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: ChatCompletionResponse =
            serde_json::from_str(&text_data).map_err(eyre::Report::from)?;
        Ok(completion)
    }
}

#[derive(Debug)]
pub struct MoonshotClient {
    api_key: String,
    api_url: String,
    client: HttpClient,
}

impl MoonshotClient {
    pub fn new(config: &MoonshotConfig) -> Self {
        let client = if config.proxy {
            HttpClient::new()
        } else {
            HttpClient::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| HttpClient::new())
        };

        Self {
            api_key: config.api_key.clone(),
            api_url: config.api_url.clone(),
            client,
        }
    }
}

#[async_trait]
impl ChatClient for MoonshotClient {
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        println!("===========self.api_key: {}", self.api_key);
        println!("{request:?}");
        let response = self
            .client
            .post(&format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("Received response: {}", error_text);
            return Err(eyre!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: ChatCompletionResponse =
            serde_json::from_str(&text_data).map_err(eyre::Report::from)?;
        Ok(completion)
    }
}

#[derive(Debug)]
pub struct DoraClient {
    output: String,
    event_sender: mpsc::Sender<ServerEvent>,
}

impl DoraClient {
    pub fn new(config: &DoraConfig, event_sender: mpsc::Sender<ServerEvent>) -> Self {
        Self {
            output: config.output.clone(),
            event_sender,
        }
    }
}

#[async_trait]
impl ChatClient for DoraClient {
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let (tx, rx) = oneshot::channel();
        self.event_sender
            .send(ServerEvent::CallNode {
                output: self.output.clone(),
                request,
                reply: tx,
            })
            .await?;
        rx.await
            .map_err(|e| eyre::eyre!("Failed to parse call tool result: {e}"))
    }
}
