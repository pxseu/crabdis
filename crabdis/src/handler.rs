use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc;

use crate::prelude::*;

pub async fn handle_client(
    stream: &mut tokio::net::TcpStream,
    session: SessionRef,
    mut rx: mpsc::UnboundedReceiver<Value>,
) -> Result<()> {
    let (mut read, mut writer) = stream.split();
    let mut reader = BufReader::new(&mut read);
    let mut writer = BufWriter::new(&mut writer);

    // Handle both reading and writing in the same task
    loop {
        tokio::select! {
            biased;
            // Handle incoming messages from the channel
            Some(value) = rx.recv() => {
                #[cfg(debug_assertions)]
                log::debug!("Received message from client: {value:?}");

                session.versioned_response(&value, &mut writer).await?;
            }

             // Handle incoming requests from the client
            result = Resp::try_parse(&mut reader, session.get_proto_version()) => {
                match result?.await? {
                    Some(Value::Multi(args)) => {
                        #[cfg(debug_assertions)]
                        log::debug!("Received command: {args:?} from session: {:?}", session.id);

                        let mut args = Args::new(&args);

                        session
                            .state
                            .handler
                            .handle_command(&mut writer, &mut args, session.clone())
                            .await?;
                    }
                    None => {
                        #[cfg(debug_assertions)]
                        log::debug!("Received empty request from session or stream closed: {:?}", session.id);
                        return Ok(());
                    }
                    _ => {
                        session
                            .versioned_response(&value_error!("Invalid request"), &mut writer)
                            .await?;
                    }
                }
            }
        }

        writer.flush().await?;
    }
}
