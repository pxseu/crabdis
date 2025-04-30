use tokio::io::{AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::prelude::*;

pub async fn handle_client(stream: &mut tokio::net::TcpStream, session: SessionRef) -> Result<()> {
    let (mut read, mut writer) = stream.split();
    let mut reader = BufReader::new(&mut read);

    // Create a channel for this client
    let (tx, mut rx) = mpsc::channel(1000);

    // Store the sender in the session
    session.set_sender(tx).await;

    // Handle both reading and writing in the same task
    loop {
        tokio::select! {
            // Handle incoming messages from the channel
            Some(value) = rx.recv() => {
                if let Err(e) = session.versioned_response(&value, &mut writer).await {
                    log::error!("Failed to write to client: {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    log::error!("Failed to flush to client: {}", e);
                    break;
                }
            }
            // Handle incoming requests from the client
            request = Value::from_resp(&mut reader) => {
                log::debug!("Received request: {:?}", request);

                match request? {
                    Some(Value::Multi(mut args)) => {
                        log::debug!("Received command: {:?} from session: {}", args, session.id);

                        session
                            .state
                            .handler
                            .handle_command(&mut writer, &mut args, session.clone())
                            .await?;
                    }
                    None => {
                        return Ok(());
                    }
                    _ => {
                        session
                            .versioned_response(&value_error!("Invalid request"), &mut writer)
                            .await?;
                    }
                }
                writer.flush().await?;
            }
        }
    }

    Ok(())
}
