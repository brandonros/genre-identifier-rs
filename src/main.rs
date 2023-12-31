mod db;
mod llms;
mod concurrency;
mod rate_limit;
mod structs;
mod retry;
mod logger;

use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;

use once_cell::sync::Lazy;
use regex::Regex;
use tokio::runtime;
use tokio::sync::Mutex;

#[cfg(feature = "openai")]
use crate::llms::openai::*;

#[cfg(feature = "llama2")]
use crate::llms::llama2::*;

use crate::rate_limit::RateLimiterWrapper;
use crate::structs::*;

const CONCURRNECY_LIMIT: usize = 1;
const MAX_REQUESTS_PER_SECOND: u32 = 30;

static RATE_LIMITER: Lazy<RateLimiterWrapper> = Lazy::new(|| {
    return RateLimiterWrapper::new(MAX_REQUESTS_PER_SECOND);
});

#[cfg(feature = "openai")]
static OPENAI_CLIENT: Lazy<OpenAIChatCompletionClient> = Lazy::new(|| {
    let model = String::from("gpt-3.5-turbo");
    let api_key = std::env::var("OPENAI_API_KEY").unwrap();
    return OpenAIChatCompletionClient::new(api_key, model);
});

#[cfg(feature = "llama2")]
static REPLICATE_CLIENT: Lazy<ReplicatePredictionClient> = Lazy::new(|| {
    let version = String::from("df7690f1994d94e96ad9d568eac121aecf50684a0b0963b25a41cc40061269e5");
    let api_key = std::env::var("REPLICATE_API_TOKEN").unwrap();
    return ReplicatePredictionClient::new(api_key, version);
});

static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    return Mutex::new(db::load_state("db.json").unwrap());
});

#[cfg(feature = "openai")]
async fn openai_process_line(line: String) -> anyhow::Result<()> {
    log::info!("Processing line: {}", &line);

    // check cache
    let state = STATE.lock().await;
    if state.results.contains_key(&line) {
        log::info!("skipping due to cache");
        return Ok(());
    }
    drop(state);

    // process
    let result = retry::retry_wrapper(100, 3, &|| async {
        // Rate limit.
        RATE_LIMITER.wait().await;
        // Settings.
        let message_content = format!("The song '{line}' is of which genre? Respond in JSON using fields `artist`, `genre`, and `track_title`.");
        return OPENAI_CLIENT.chat_completion(&message_content).await;
    }).await;
    if result.is_err() {
        log::error!("failed to get response after retries, skipping");
        log::error!("err = {:?}", result.err());
        log::error!("line = {}", line);
        return Ok(());
    }
    let response_body: ChatCompletionResponse = result.unwrap();

    // increment tokens
    let mut state = STATE.lock().await;
    state.total_completion_tokens += response_body.usage.completion_tokens;
    state.total_prompt_tokens += response_body.usage.prompt_tokens;
    state.total_tokens += response_body.usage.total_tokens;

    // calculate costs
    let input_token_cost_per_1000 = 0.0015;
    let output_token_cost_per_1000 = 0.002;
    let total_input_cost = (state.total_prompt_tokens as f64 / 1000.0) * input_token_cost_per_1000;
    let total_output_cost = (state.total_completion_tokens as f64 / 1000.0) * output_token_cost_per_1000;
    let total_cost = total_input_cost + total_output_cost;

    // log tokens and costs
    log::info!("total_completion_tokens = {} total_prompt_tokens = {} total_tokens = {}", state.total_completion_tokens, state.total_prompt_tokens, state.total_tokens);
    log::info!("total_input_cost = ${total_input_cost:.4} total_output_cost = ${total_output_cost:.4} total_cost = ${total_cost:.4}");

    // parse response message
    let response_message = &response_body.choices[0].message.content;
    let parse_result = serde_json::from_str::<Song>(&response_message);
    if parse_result.is_err() {
        log::error!("failed to parse response, skipping");
        log::error!("err = {:?}", parse_result.err());
        log::error!("line = {}", line);
        return Ok(());
    }
    let parsed_response_message = parse_result.unwrap();

    // log message
    log::info!("result: {},{}", line, serde_json::to_string(&parsed_response_message)?);

    // push result
    state.results.insert(line, parsed_response_message);

    // save state
    db::save_state(&*state, "db.json")?;

    return Ok(());
}

#[cfg(feature = "llama2")]
async fn llama2_process_line(line: String) -> anyhow::Result<()> {
    log::info!("Processing line: {}", &line);

    // check cache
    let state = STATE.lock().await;
    if state.results.contains_key(&line) {
        log::info!("skipping due to cache");
        return Ok(());
    }
    drop(state);

    // process
    let result = retry::retry_wrapper(100, 3, &|| async {
        // Rate limit.
        RATE_LIMITER.wait().await;
        // Settings.
        let message_content = format!("User: Parse this string `{line}` which represents a song + artist information into {{\"artists\": ..., \"genre\": ..., \"track_title\": ...}}\nAssistant: \n");
        return REPLICATE_CLIENT.predict(&message_content).await;
    }).await;
    if result.is_err() {
        log::error!("failed to get response after retries, skipping");
        log::error!("err = {:?}", result.err());
        log::error!("line = {}", line);
        return Ok(());
    }
    let response_body: PredictionResponse = result.unwrap();

    // parse response message
    let response_tokens = &response_body.output.unwrap();
    let response_message = response_tokens.join(""); // combine tokens
    log::info!("line = {line} response_message = {response_message}");
    let re = Regex::new(r"(\{.*\})")?;
    let captures = re.captures(&response_message);
    if captures.is_none() {
        log::error!("line = {line} captures.is_none()");
        return Ok(());
    }
    let captures = captures.unwrap();
    if captures.len() == 0 {
        log::error!("line = {line} captures.len() == 0");
        return Ok(());
    }
    let capture = &captures[0];
    let parse_result = serde_json::from_str::<Song>(&capture);
    if parse_result.is_err() {
        log::error!("line = {line} parse_result.is_err()");
        return Ok(());
    }
    let parsed_response_message = parse_result.unwrap();

    // log message
    log::info!("result: {},{}", line, serde_json::to_string(&parsed_response_message)?);

    // push result
    let mut state = STATE.lock().await;
    state.results.insert(line, parsed_response_message);

    // save state
    db::save_state(&*state, "db.json")?;

    return Ok(());
}

fn main() -> anyhow::Result<()> {
    // init logger
    logger::init_stdout_logger();

    // build runtime
    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // spawn async task into runtime
    return runtime.block_on(async {
        // open file + read lines into Vec<String>
        let args: Vec<String> = std::env::args().collect();
        let path = Path::new(&args[1]);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        // process each line concurrently
        concurrency::concurrency_wrapper(lines, CONCURRNECY_LIMIT, |line| async move {
            #[cfg(feature = "llama2")]
            {
                return llama2_process_line(line).await;
            }
            #[cfg(feature = "openai")]
            {
                return openai_process_line(line).await;
            }
            #[cfg(not(any(feature = "llama2", feature = "openai")))]
            compile_error!("Either feature llama2 or openai must be enabled.");
        }).await?;
        Ok(())
    });
}
