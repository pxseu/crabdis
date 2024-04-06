use crabdis::error::Result;
use crabdis::storage::value::Value;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    let mut stream = TcpStream::connect("localhost:6379").await?;
    let (mut reader, mut writer) = stream.split();
    let mut bufreader = BufReader::new(&mut reader);

    for i in 0..1_000_000 {
        let req = Value::Multi(
            vec![
                Value::String("SET".into()),
                Value::String(format!("key{i}")),
                Value::String(format!("value{i}")),
            ]
            .into(),
        );

        println!("Sending request: {req:?}");

        req.to_resp2(&mut writer).await?;

        writer.flush().await?;

        // can return none if the client has disconnected
        let Some(resp) = Value::from_resp(&mut bufreader).await? else {
            return Ok(());
        };

        println!("Received response: {resp:?}");

        assert_eq!(resp, Value::Ok);
    }

    Ok(())
}
