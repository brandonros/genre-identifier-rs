mod db;
mod openai;
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
use tokio::runtime;
use tokio::sync::Mutex;
use crate::openai::OpenAIChatCompletionClient;
use crate::rate_limit::RateLimiterWrapper;
use crate::structs::*;

const CONCURRNECY_LIMIT: usize = 16;
const MAX_REQUESTS_PER_SECOND: u32 = 30;

static RATE_LIMITER: Lazy<RateLimiterWrapper> = Lazy::new(|| {
    return RateLimiterWrapper::new(MAX_REQUESTS_PER_SECOND);
});

static OPENAI_CLIENT: Lazy<OpenAIChatCompletionClient> = Lazy::new(|| {
    let model = String::from("gpt-3.5-turbo");
    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap();
    return OpenAIChatCompletionClient::new(openai_api_key, model);
});

static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    return Mutex::new(db::load_state("db.json").unwrap());
});

async fn process_line(line: String) -> anyhow::Result<()> {
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
    let response_body = result.unwrap();

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
            return process_line(line).await;
        }).await?;
        Ok(())
    });
}
