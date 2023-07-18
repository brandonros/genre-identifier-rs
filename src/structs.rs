use std::collections::HashMap;

use serde::{Serialize, Deserialize};

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
