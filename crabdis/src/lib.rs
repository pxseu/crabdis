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
mod traits;
mod utils;

use std::net::{IpAddr, SocketAddr};
use std::num::{NonZeroU16, NonZeroUsize};
use std::path::PathBuf;

use clap::Parser;
use crabdis_core::shutdown::Receiver;
use tokio::net::TcpListener;

use self::prelude::*;
use crate::handler::handle_client;
use crate::session::state::State;
use crate::storage::rdb;

// This is really silly because Windows does support IPv6, but binding to "::",
// doesn't default to dual-stack mode. Shame!
#[cfg(unix)]
const DEFAULT_ADDRESS: &str = "::";
#[cfg(not(unix))]
const DEFAULT_ADDRESS: &str = "0.0.0.0";

#[derive(Parser, Clone)]
pub struct CLI {
    /// Address to bind to
    #[clap(short, long, default_value = DEFAULT_ADDRESS)]
    pub address: IpAddr,

    /// Port to listen on
    #[clap(short, long, default_value = "6379")]
    pub port: NonZeroU16,

    #[clap(short, long, default_value = "1")]
    pub threads: NonZeroUsize,

    /// Enable verbose logging
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

    /// Password for authentication
    #[clap(long = "requirepass")]
    pub password: Option<String>,
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

    let state = State::new(&cli);

    let listener = TcpListener::bind(SocketAddr::new(cli.address, cli.port.get())).await?;

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
                log::warn!("Received {signal}, preparing to shut down...");

                // Perform final save if RDB is enabled
                if state.rdb_config.is_enabled() {
                    if let Err(e) = rdb::save_rdb(&state).await {
                        log::error!("Failed to save RDB on shutdown: {e}");
                        return Err(e);
                    }
                    log::info!("Data saved successfully");
                }

                log::info!("bye bye~");
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
