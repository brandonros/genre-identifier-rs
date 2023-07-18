use futures::Future;

pub async fn retry_wrapper<Fut, T>(base: u64, num_tries: usize, cb: &impl Fn() -> Fut) -> anyhow::Result<T>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let retry_strategy = tokio_retry::strategy::ExponentialBackoff::from_millis(base)
        .map(tokio_retry::strategy::jitter) // add jitter to delays
        .take(num_tries);
    return tokio_retry::Retry::spawn(retry_strategy, || async {
        return cb().await;
    }).await;
}
