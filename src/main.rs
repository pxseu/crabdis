#[tokio::main]
async fn main() -> crabdis::error::Result<()> {
    let _ = crabdis::run().await?;

    Ok(())
}
