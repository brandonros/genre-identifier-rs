use reqwest::{Client, header};
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct Usage {
    pub completion_tokens: usize,
    pub prompt_tokens: usize,
    pub total_tokens: usize
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub role: String
}

#[derive(Serialize, Deserialize)]
pub struct Choice {
    pub finish_reason: String,
    pub index: usize,
    pub message: Message
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub created: usize,
    pub model: String,
    pub object: String,
    pub usage: Usage,
    pub choices: Vec<Choice>
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub max_tokens: Option<usize>,
    pub model: String,
    pub messages: Vec<Message>
}

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
        let stringified_request_body = serde_json::to_string(&request_body)?;

        let mut request_headers = header::HeaderMap::new();
        request_headers.insert("Authorization", header::HeaderValue::from_str(&format!("Bearer {}", self.api_key))?);
        request_headers.insert("Content-Type", header::HeaderValue::from_str("application/json")?);

        // Perform request.
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .headers(request_headers)
            .timeout(Duration::from_millis(5000))
            .body(stringified_request_body)
            .send()
            .await?;

        let stringified_response_body = response.text().await?;
        log::debug!("stringified_response_body = {stringified_response_body}");

        let response_body: ChatCompletionResponse = serde_json::from_str(&stringified_response_body)?;

        Ok(response_body)
    }
}
