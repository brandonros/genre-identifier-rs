use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{Write, Read};

pub fn save_state<T: Serialize>(state: &T, file_path: &str) -> anyhow::Result<()> {
    let mut file = File::create(file_path)?;
    let json = serde_json::to_string_pretty(state)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn load_state<T: for<'de> Deserialize<'de>>(file_path: &str) -> anyhow::Result<T> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let state: T = serde_json::from_str(&contents)?;
    Ok(state)
}
