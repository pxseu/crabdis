use clap::Parser;
use crabdis::CLI;

fn main() -> crabdis::error::Result<()> {
    let cli = CLI::parse();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(cli.threads.get())
        .enable_io()
        .enable_time()
        .build()?
        .block_on(async { crabdis::run(cli).await })
}
