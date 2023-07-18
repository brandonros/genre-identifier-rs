use futures::{Future, StreamExt, TryStreamExt};

pub async fn concurrency_wrapper<Fut, T, F>(rows: Vec<T>, limit: usize, cb: F) -> anyhow::Result<()>
where
    T: Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + Copy + 'static,
{
    return futures::stream::iter(rows)
        .map(Ok) // TODO: do not understand why .map(Ok)
        .try_for_each_concurrent(limit,|row| async move {
            return cb(row).await;
        }
    ).await;
}
