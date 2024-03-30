use crabdis::error::Result;
use crabdis::storage::value::Value;
use crabdis::CLI;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = CLI {
        address: [127, 0, 0, 1].into(),
        port: 6379,
        threads: 1,
    };

    let connect_address = format!("{}:{}", cli.address, cli.port);

    let mut stream = TcpStream::connect(connect_address).await?;
    let (mut reader, mut writer) = stream.split();
    let mut bufreader = BufReader::new(&mut reader);

    for i in 0..1000 {
        let req = Value::Multi(
            vec![
                Value::String("SET".into()),
                Value::String(format!("key{i}")),
                Value::String(format!("value{i}")),
            ]
            .into(),
        );

        println!("Sending request: {req:?}");

        req.to_resp(&mut writer).await?;

        writer.flush().await?;

        let Some(resp) = Value::from_resp(&mut bufreader).await? else {
            return Ok(());
        };

        println!("Received response: {resp:?}");

        assert_eq!(resp, Value::Ok);
    }

    Ok(())
}
