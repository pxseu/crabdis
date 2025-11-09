use std::pin::Pin;

use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use crate::CLI;
use crate::prelude::*;

pub mod logger;

pub fn bootlog(cli: &CLI) {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let bits = std::mem::size_of::<usize>() * 8;
    let pid = std::process::id();
    let port = cli.port;
    let threads = cli.threads;

    log::info!("{name} is starting");
    log::info!("version={version}, bits={bits}, pid={pid}, threads={threads}");

    println!(
        r#"
    ⣿⣿⣿⡿⠋⠀⠉⠛⢿⣿⣿⣿⣿⣿⣿⣿⠟⠉⠀⡉⢻⣿⣿⣿⣿⣿
    ⣿⣿⢏⠞⡔⢠⣄⠀⠀⠙⢿⣿⣿⣿⣿⠃⠀⠀⠀⣿⠀⠻⣎⢿⣿⣿
    ⣿⡏⠞⣰⠃⣾⣿⣷⣄⠀⠀⠙⠿⠿⠃⠀⢀⣴⣷⢸⣆⠀⠹⣷⣻⣿
    ⡿⡰⠇⠀⣸⣿⣿⣿⠟⢁⡄⠀⢀⠀⠀⠀⠈⢻⣿⡞⣿⡄⠀⠹⣷⢿      {name} {version} (00000000/0) {bits} bit
    ⡇⠉⠀⣰⣿⣿⡿⠁⠔⠁⢡⠈⠀⠀⠀⠀⠀⠀⢹⣿⡘⣷⡀⠀⠹⣾
    ⣧⣀⣠⣿⣿⣿⢁⠀⢀⢀⣿⣧⣼⣷⠶⢀⠀⠀⠀⢿⣷⣜⠳⠀⢠⣿      Running in standalone mode
    ⣿⣿⣿⣿⣿⣿⢸⠀⠀⢨⣀⣽⣿⣿⣦⣴⠀⠂⠀⢸⣿⣿⣿⣿⣿⣿      Threads: {threads}
    ⣿⣿⣿⣿⣿⣿⡼⡇⠀⠘⣿⣿⣿⣿⣿⣇⠀⠀⠀⠘⣿⣿⣿⣿⣿⣿      Port: {port}
    ⣿⣿⣿⣿⣿⣿⣿⠃⡄⠀⠘⠻⠿⠿⣛⡅⠆⠀⠀⠀⢻⣿⣿⣿⣿⣿      PID: {pid}
    ⣿⣿⣿⣿⣿⣿⣿⠀⠁⠀⠀⢀⢬⣻⢿⣇⢀⣀⣀⡀⠀⢻⣿⣿⣿⣿
    ⣿⣿⣿⣿⣿⣿⡇⢀⣾⣿⣧⡜⣭⣿⠁⣀⢀⣼⣿⣿⡆⠈⢿⣿⣿⣿
    ⣿⣿⣿⣿⣿⡿⠀⢸⣿⣿⣿⢻⣿⣿⣷⣿⣿⣿⣿⣿⡻⠀⡘⣿⣿⣿
    ⣿⣿⣿⣿⣿⠃⠀⠸⣿⣿⣿⣿⠟⣙⡻⢿⣮⢿⣿⡟⡡⡄⠡⢹⣿⣿
    ⣿⣿⣿⣿⣿⢰⠀⡆⣿⣿⣿⡟⣸⣆⣿⣷⠍⠻⠏⠴⠿⠃⠀⠈⣿⣿
    ⣿⣿⣿⣿⣿⢸⡄⢁⢸⣿⣿⡇⠘⠛⠉⠁⠀⠀⠀⠀⠀⠀⠀⡇⣿⣿
    ⣿⣿⣿⣿⣿⣿⡗⠀⠀⢿⣿⡇⠀⠀⠀⠀⠀⠀⠀⢠⢠⠂⠀⣷⣿⣿
"#,
    );
}

pub async fn can_read<R>(reader: &mut R) -> Result<bool>
where
    R: AsyncBufRead + Unpin,
{
    Ok(!reader.fill_buf().await?.is_empty())
}

pub async fn try_parse<'a, R>(
    reader: &'a mut R,
) -> Result<Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send + 'a>>>
where
    R: AsyncBufRead + Unpin + Send + 'a,
{
    if !can_read(reader).await? {
        return Ok(Box::pin(async move { Ok(None) }));
    }

    Ok(Value::from_resp(reader))
}
