use tokio::time::{Duration, Interval, interval as tokio_interval};

pub async fn interval(seconds: u64) -> Interval {
    let mut i = tokio_interval(Duration::from_secs(seconds));
    // skip the first immediate tick
    i.tick().await;
    i
}
