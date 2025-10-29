use clap::Parser;
use crabdis::error::Result;
use crabdis::storage::value::Value;
use tokio::io::{ AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = crabdis::run(crabdis::CLI::parse_from([
            "crabdis",
            "--address",
            "127.0.0.1",
            "--port",
            "6379",
        ]))
        .await
        {
            println!("Failed to start crabdis: {e}");
        } else {
            println!("Crabdis started successfully");
        }
    });

    let mut stream = TcpStream::connect("localhost:6379").await?;
    let (mut reader, mut writer) = stream.split();
    let mut reader = BufReader::new(&mut reader);
    let mut writer = BufWriter::new(&mut writer);

    let start = tokio::time::Instant::now();
    for i in 0..1_000_000 {
        Value::Multi(
            vec![
                Value::String("SET".into()),
                Value::String(format!("key{i}").into()),
                Value::String(format!("value{i}").into()),
            ]
            .into(),
        )
        .to_resp2(&mut writer)
        .await?;
    }

    writer.flush().await?;
    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);

    let mut count = 0;

    let start = tokio::time::Instant::now();
    for _ in 0..1_000_000 {
        if let Some(Value::Ok) = Value::from_resp(&mut reader).await? {
            count += 1;
        }
    }
    let duration = start.elapsed();
    println!("Time taken: {:?}, count: {}", duration, count);

    Ok(())
}
