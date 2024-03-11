mod commands;
pub mod error;
mod handler;
mod prelude;
mod storage;
mod utils;

use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use self::prelude::*;
use crate::handler::handle_client;

#[derive(Parser)]
pub struct CLI {
    #[clap(short, long)]
    pub address: Option<IpAddr>,

    #[clap(short, long)]
    pub port: Option<u16>,
}

pub async fn run() -> Result<()> {
    let cli = CLI::parse();
    utils::log::init(cfg!(debug_assertions));

    let store = Store::new();
    let listener = TcpListener::bind(SocketAddr::new(
        cli.address
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        cli.port.unwrap_or(6379),
    ))
    .await?;

    log::info!("Listening on {}", listener.local_addr()?);

    loop {
        let (mut stream, _) = listener.accept().await?;
        let store = store.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(&mut stream, store).await {
                log::error!("Error: {e}");
                stream.shutdown().await.ok();
            }
        });
    }
}
