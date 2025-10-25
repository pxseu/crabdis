mod commands;
pub mod error;
mod handler;
mod prelude;
mod session;
pub mod storage;
mod utils;

use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use self::prelude::*;
use crate::handler::handle_client;
use crate::session::state::State;

#[derive(Parser)]
pub struct CLI {
    #[clap(short, long, default_value = "::")]
    pub address: IpAddr,

    #[clap(short, long, default_value = "6379")]
    pub port: u16,

    #[clap(short, long, default_value = "1")]
    pub threads: usize,

    #[clap(short, long, default_value = "false")]
    pub verbose: bool,
}

pub async fn run(cli: CLI) -> Result<()> {
    utils::logger::init(cfg!(debug_assertions) || cli.verbose);

    let state = State::new().await;

    let listener = TcpListener::bind(SocketAddr::new(cli.address, cli.port)).await?;

    utils::bootlog(&cli);

    log::info!(
        "Listening on {}",
        listener
            .local_addr()
            .context("Failed to get local address")?
    );

    loop {
        #[cfg(debug_assertions)]
        let (mut stream, addr) = listener
            .accept()
            .await
            .context("Failed to accept connection")?;

        #[cfg(not(debug_assertions))]
        let (mut stream, _) = listener.accept().await?;

        let state = state.clone();

        tokio::spawn(async move {
            use std::io::ErrorKind;

            // Disable Nagle's algorithm to ensure immediate delivery of data
            if let Err(e) = stream.set_nodelay(true) {
                log::error!("Failed to set TCP_NODELAY: {e}");
                return;
            }

            let session = state.new_session().await;

            #[cfg(debug_assertions)]
            log::debug!(
                "Accepted connection from {addr} for session: {}",
                session.id
            );
            if let Err(e) = handle_client(&mut stream, session.clone()).await {
                match e {
                    Error::Io(e)
                        if matches!(
                            e.kind(),
                            ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset
                        ) => {}
                    _ => log::error!("Unkown connection error: {e:?}"),
                }
            }

            stream.shutdown().await.ok();
            #[cfg(debug_assertions)]
            log::debug!("Session closed: {}", session.id);
            session.cleanup().await;
            #[cfg(debug_assertions)]
            log::debug!("Connection from {addr} closed");
        });
    }
}
