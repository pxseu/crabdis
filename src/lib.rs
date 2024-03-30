mod commands;
mod context;
pub mod error;
mod handler;
mod prelude;
pub mod storage;
mod utils;

use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use self::prelude::*;
use crate::context::Context;
use crate::handler::handle_client;

#[derive(Parser)]
pub struct CLI {
    #[clap(short, long, default_value = "0.0.0.0")]
    pub address: IpAddr,

    #[clap(short, long, default_value = "6379")]
    pub port: u16,

    #[clap(short, long, default_value = "1")]
    pub threads: usize,
}

pub async fn run(cli: CLI) -> Result<()> {
    utils::logger::init(cfg!(debug_assertions));

    utils::bootlog(&cli);

    let context = Context::new().await;

    let listener = TcpListener::bind(SocketAddr::new(cli.address, cli.port)).await?;

    log::info!("Listening on {}", listener.local_addr()?);

    loop {
        let (mut stream, _addr) = listener.accept().await?;
        #[cfg(debug_assertions)]
        log::debug!("Accepted connection from {_addr}");
        let context = context.clone();

        tokio::spawn(async move {
            use std::io::ErrorKind;

            if let Err(e) = handle_client(&mut stream, context).await {
                match e {
                    Error::Io(e)
                        if matches!(
                            e.kind(),
                            ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset
                        ) => {}
                    _ => log::error!("Error: {e:?}"),
                }

                stream.shutdown().await.ok();

                #[cfg(debug_assertions)]
                log::debug!("Connection from {_addr} closed");
            }
        });
    }
}
