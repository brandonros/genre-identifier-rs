use reqwest::{Client, header};
use std::time::Duration;
use crate::structs::*;

pub struct OpenAIChatCompletionClient {
    client: Client,
    model: String,
    api_key: String,
}

impl OpenAIChatCompletionClient {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .build()
            .unwrap();
        Self {
            client,
            model,
            api_key
        }
    }

    pub async fn chat_completion(&self, message_content: &str) -> anyhow::Result<ChatCompletionResponse> {
        // Prepare the API request
        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            max_tokens: None, // TODO: cap at 60?
            messages: vec![
                Message {
                    role: String::from("user"),
                    content: message_content.to_string()
                }
            ]
        };

        let mut request_headers = header::HeaderMap::new();
        request_headers.insert("Authorization", header::HeaderValue::from_str(&format!("Bearer {}", self.api_key))?);

        // Perform request.
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .headers(request_headers)
            .timeout(Duration::from_millis(5000))
            .json(&request_body)
            .send()
            .await?;

        let stringified_response_body = response.text().await?;
        log::debug!("stringified_response_body = {stringified_response_body}");

        let response_body: ChatCompletionResponse = serde_json::from_str(&stringified_response_body)?;

        Ok(response_body)
    }
}
