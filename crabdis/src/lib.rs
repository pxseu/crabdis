#![deny(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(
    clippy::multiple_crate_versions,
    clippy::significant_drop_tightening,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod commands;
pub mod error;
mod handler;
mod prelude;
mod session;
mod storage;
mod utils;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use clap::Parser;
use crabdis_core::shutdown::Receiver;
use tokio::net::TcpListener;

use self::prelude::*;
use crate::handler::handle_client;
use crate::session::state::State;

#[derive(Parser, Clone)]
pub struct CLI {
    #[clap(short, long, default_value = "::")]
    pub address: IpAddr,

    #[clap(short, long, default_value = "6379")]
    pub port: u16,

    #[clap(short, long, default_value = "1")]
    pub threads: usize,

    #[clap(short, long, default_value = "false")]
    pub verbose: bool,

    /// Directory where RDB file is stored
    #[clap(long, default_value = "./")]
    pub dir: PathBuf,

    /// RDB filename
    #[clap(long, default_value = "dump.rdb")]
    pub dbfilename: String,

    /// Save points in format "seconds changes" (e.g., "900 1" saves after 900s
    /// if 1+ keys changed). Can be specified multiple times. Pass --save ""
    /// to disable RDB persistence.
    #[clap(long = "save", value_name = "SECONDS CHANGES")]
    pub save_points: Vec<String>,
}

/// Runs the Crabdis server with the given CLI configuration.
///
/// # Errors
///
/// Returns an error if binding to the specified address fails or if the
/// server encounters an unrecoverable I/O error.
pub async fn run(cli: CLI, mut shutdown_rx: Receiver) -> Result<()> {
    utils::logger::init(cfg!(debug_assertions) || cli.verbose);

    utils::bootlog(&cli);

    let state = State::new(&cli).await;

    let listener = TcpListener::bind(SocketAddr::new(cli.address, cli.port)).await?;

    log::info!(
        "Listening on {}",
        listener
            .local_addr()
            .context("Failed to get local address")?
    );

    loop {
        tokio::select! {
            // biased, since we want to prioritize shutdown signals and remove randomness overhead
            biased;

            signal = shutdown_rx.recv() => {
                let signal = signal.context("Failed to receive shutdown signal")?;
                log::warn!("Received {signal}");

                // Perform final save if RDB is enabled
                if state.rdb_config.enabled {
                    if let Err(e) = state.save_rdb().await {
                        log::error!("Failed to save RDB on shutdown: {e}");
                        return Err(e);
                    }
                    log::info!("Data saved successfully");
                }

                log::info!("Shutting down gracefully");
                return Ok(());
            }

            result = listener.accept() => {
                let (stream, addr) = result.context("Failed to accept connection")?;
                let state = state.clone();

                tokio::spawn(handle_client(stream, state, addr));
            }
        }
    }
}
