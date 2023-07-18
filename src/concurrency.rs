use futures::{Future, StreamExt, TryStreamExt, stream};

pub async fn concurrency_wrapper<Fut, T, F>(rows: Vec<T>, limit: usize, cb: F) -> anyhow::Result<()>
where
    T: Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + Copy + 'static,
{
    let stream = stream::iter(rows);
    return stream
        .map(Ok)
        .try_for_each_concurrent(limit,|row| async move {
            return cb(row).await;
        }
    ).await;
}
