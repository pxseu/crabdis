pub mod error;
pub mod handler;
pub mod listiner;
pub mod prelude;
pub mod storage;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use self::listiner::TcpListener;
use self::prelude::*;

async fn handle_client(mut stream: TcpStream, _store: Store) {
    stream.write("+OK\r\n".as_bytes()).await.unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let store = Store::new();
    let listener = TcpListener::bind(None, None).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let store = store.clone();

        tokio::spawn(async move {
            let _ = handle_client(stream, store).await;
        });
    }
}
