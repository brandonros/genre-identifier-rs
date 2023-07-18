use reqwest::{Client, header};
use serde::{Serialize, Deserialize};
use std::{time::Duration, collections::HashMap};

#[derive(Serialize, Deserialize)]
pub struct PredictionInput {
    pub prompt: String
}

#[derive(Serialize, Deserialize)]
pub struct PredictionRequest {
    pub version: String,
    pub input: PredictionInput
}

#[derive(Serialize, Deserialize)]
pub struct PredictionResponse {
    pub id: String,
    pub version: String,
    pub input: PredictionInput,
    pub logs: String,
    pub error: Option<String>,
    pub status: String,
    pub created_at: String,
    pub urls: HashMap<String, String>,
    pub output: Option<Vec<String>>,
}

pub struct ReplicatePredictionClient {
    client: Client,
    api_key: String,
    version: String,
}

impl ReplicatePredictionClient {
    pub fn new(api_key: String, version: String) -> Self {
        let client = Client::builder()
            .build()
            .unwrap();
        Self {
            client,
            api_key,
            version
        }
    }

    async fn start_prediction(&self, message_content: &str) -> anyhow::Result<PredictionResponse> {
        // request headers
        let mut request_headers = header::HeaderMap::new();
        request_headers.insert("Authorization", header::HeaderValue::from_str(&format!("Token {}", self.api_key))?);
        request_headers.insert("Content-Type", header::HeaderValue::from_str("application/json")?);
        // request body
        let request_body = PredictionRequest {
            version: self.version.clone(),
            input: PredictionInput {
                prompt: message_content.to_string()
            }
        };
        let stringified_request_body = serde_json::to_string(&request_body)?;
        // response
        let response = self.client
            .post("https://api.replicate.com/v1/predictions")
            .headers(request_headers)
            .timeout(Duration::from_millis(5000))
            .body(stringified_request_body)
            .send()
            .await?;
        let stringified_response_body = response.text().await?;
        log::debug!("stringified_response_body = {stringified_response_body}");
        let response_body: PredictionResponse = serde_json::from_str(&stringified_response_body)?;
        return Ok(response_body);
    }

    async fn poll_prediction(&self, prediction_id: &str) -> anyhow::Result<PredictionResponse> {
        loop {
            // request headers
            let mut request_headers = header::HeaderMap::new();
            request_headers.insert("Authorization", header::HeaderValue::from_str(&format!("Token {}", self.api_key))?);
            request_headers.insert("Content-Type", header::HeaderValue::from_str("application/json")?);
            // response
            let url = format!("https://api.replicate.com/v1/predictions/{}", prediction_id);
            let response = self.client
                .get(&url)
                .headers(request_headers.clone())
                .timeout(Duration::from_millis(5000))
                .send()
                .await?;
            let stringified_response_body = response.text().await?;
            log::debug!("stringified_response_body = {stringified_response_body}");
            let response_body: PredictionResponse = serde_json::from_str(&stringified_response_body)?;
            if response_body.status == "succeeded" {
                return Ok(response_body);
            } else if response_body.status == "processing" || response_body.status == "starting" {
                log::debug!("status = {}", response_body.status);
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            } else {
                panic!("unknown status: {}", response_body.status);
            }
        }
    }

    pub async fn predict(&self, message_content: &str) -> anyhow::Result<PredictionResponse> {
        let prediction = self.start_prediction(message_content).await?;
        return self.poll_prediction(&prediction.id).await;
    }
}
