use tokio::io::AsyncWriteExt;

use crate::commands::handle_command;
use crate::prelude::*;

pub async fn handle_client(
    stream: &mut tokio::net::TcpStream,
    mut store: crate::storage::Store,
) -> Result<()> {
    let (mut read, mut writer) = stream.split();

    let mut reader = tokio::io::BufReader::new(&mut read);

    loop {
        let request = Value::from_resp(&mut reader).await?;

        #[cfg(debug_assertions)]
        log::debug!("Received request: {request:?}");

        let mut args = match request {
            Value::Multi(args) => args,
            _ => {
                writer
                    .write(&Value::Error("Invalid request".to_string()).bytes())
                    .await?;
                continue;
            }
        };

        let command = match args.pop_front() {
            Some(Value::String(command)) => command,
            _ => {
                writer
                    .write(&Value::Error("Invalid command".to_string()).bytes())
                    .await?;
                continue;
            }
        };

        let response = handle_command(&command, &mut args, &mut store).await?;

        #[cfg(debug_assertions)]
        log::debug!("Sending response: {response:?}");

        writer.write(&response.bytes()).await?;
    }
}
