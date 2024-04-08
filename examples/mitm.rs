use crabdis::error::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<()> {
    const REDIS_PORT: u16 = 6379;
    const FAKE_PORT: u16 = 6380;

    let fake = tokio::net::TcpListener::bind(format!("127.0.0.1:{FAKE_PORT}")).await?;

    loop {
        let (mut redis_stream, _) = fake.accept().await?;

        let mut fake_stream =
            tokio::net::TcpStream::connect(format!("127.0.0.1:{REDIS_PORT}")).await?;

        tokio::spawn(async move {
            // log all data and forward it
            let (mut redis_reader, mut redis_writer) = redis_stream.split();

            let (mut fake_reader, mut fake_writer) = fake_stream.split();

            let mut redis_buffer = vec![0; 1024];
            let mut fake_buffer = vec![0; 1024];

            loop {
                tokio::select! {
                    Ok(n) = redis_reader.read(&mut redis_buffer) => {
                        if n == 0 {
                            break;
                        }

                        let data = &redis_buffer[..n];
                        let text = std::str::from_utf8(data).unwrap();

                        println!("REDIS -> FAKE: {text}" );
                        fake_writer.write_all(data).await.unwrap();
                    }
                    Ok(n) = fake_reader.read(&mut fake_buffer) => {
                        if n == 0 {
                            break;
                        }

                        let data = &fake_buffer[..n];
                        let text = std::str::from_utf8(data).unwrap();

                        println!("FAKE -> REDIS: {text}");
                        redis_writer.write_all(data).await.unwrap();
                    }
                }
            }
        });
    }
}
