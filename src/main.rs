#![deny(clippy::pedantic, clippy::nursery, clippy::cargo)]

use clap::Parser;
use crabdis::CLI;

fn main() -> crabdis::error::Result<()> {
    let cli = CLI::parse();

    if cli.threads == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Thread count must be greater than 0",
        )
        .into());
    }

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(cli.threads)
        .enable_all()
        .build()?
        .block_on(crabdis::run(cli))
}
