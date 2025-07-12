use crate::models::{CompletionRequest, CompletionResponse};
use eyre::eyre;
use eyre::Result;
use reqwest::Client as HttpClient;
use salvo::async_trait;

use crate::config::{DeepseekConfig, GeminiConfig};
use crate::DataId;

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
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
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
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
            println!("API error: {}", error_text);
            return Err(eyre!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: CompletionResponse = serde_json::from_str(&text_data)
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
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        println!("================={:#?}", self);
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
            println!("API error: {}", error_text);
            return Err(eyre!("API Error: {}", error_text));
        }
        let text_data = response.text().await?;
        println!("Received response: {}", text_data);
        let completion: CompletionResponse = serde_json::from_str(&text_data)
            .map_err(eyre::Report::from)
            .unwrap();
        Ok(completion)
    }
}

#[derive(Debug)]
pub struct EventClient {
    node_id: DataId,
}

impl EventClient {
    pub fn new(node_id: DataId) -> Self {
        Self { node_id }
    }
}

#[async_trait]
impl ChatClient for EventClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        unimplemented!("EventClient does not support completion requests yet");
        // let text_data = response.text().await?;
        // println!("Received response: {}", text_data);
        // let completion: CompletionResponse = serde_json::from_str(&text_data)
        //     .map_err(eyre::Error::from)
        //     .unwrap();
        // Ok(completion)
    }
}
