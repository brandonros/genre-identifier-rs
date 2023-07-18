use std::collections::HashMap;

use serde::{Serialize, Deserialize};

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

#[derive(Serialize, Deserialize)]
pub struct Song {
    pub artist: String,
    pub genre: String,
    pub track_title: String
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub total_completion_tokens: usize,
    pub total_prompt_tokens: usize,
    pub total_tokens: usize,
    pub results: HashMap<String, Song>
}
