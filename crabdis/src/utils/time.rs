pub async fn interval(seconds: u64) -> tokio::time::Interval {
    let mut i = tokio::time::interval(std::time::Duration::from_secs(seconds));
    i.tick().await;
    i
}
