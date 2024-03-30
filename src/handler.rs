use tokio::io::{AsyncWriteExt, BufReader};

use crate::prelude::*;

pub async fn handle_client(stream: &mut tokio::net::TcpStream, context: ContextRef) -> Result<()> {
    let (mut read, mut writer) = stream.split();

    let mut reader = BufReader::new(&mut read);

    loop {
        let request = Value::from_resp(&mut reader).await?;

        #[cfg(debug_assertions)]
        log::debug!("Received request: {request:?}");

        match request {
            Some(Value::Multi(mut args)) => {
                context
                    .commands
                    .handle_command(&mut writer, &mut args, context.clone())
                    .await?
            }

            // If the request is None, the client has disconnected.
            None => {
                return Ok(());
            }

            _ => {
                value_error!("Invalid request").to_resp(&mut writer).await?;
            }
        };

        writer.flush().await?;
    }
}
