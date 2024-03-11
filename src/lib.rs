pub mod error;
mod handler;
mod prelude;
mod storage;
mod utils;

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use tokio::net::TcpListener;

use self::prelude::*;
use crate::handler::handle_client;

pub async fn run() -> Result<()> {
    utils::log::init(false);

    let store = Store::new();
    let listener = TcpListener::bind(SocketAddr::new(
        IpAddr::from_str("127.0.0.1").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Could not parse IP address",
            )
        })?,
        6379,
    ))
    .await?;

    log::info!("Listening on {}", listener.local_addr()?);

    loop {
        let (stream, _) = listener.accept().await?;
        let store = store.clone();

        tokio::spawn(async move {
            let _ = handle_client(stream, store).await;
        });
    }
}
