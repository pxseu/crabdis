use std::io::ErrorKind;

use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::commands::COMMANDS;
use crate::prelude::*;

pub async fn handle_client(
    mut stream: TcpStream,
    session: SessionRef,
    rx: UnboundedReceiver<Value>,
) {
    // Disable Nagle's algorithm to ensure immediate delivery of data
    if let Err(e) = stream.set_nodelay(true) {
        log::error!("Failed to set TCP_NODELAY: {e}");
        return;
    }

    #[cfg(debug_assertions)]
    log::debug!("Accepted connection from: {session:?}");

    if let Err(e) = handle_connection(&mut stream, session.clone(), rx).await {
        match e {
            Error::Io(e) | Error::Core(CoreError::Io(e))
                if matches!(
                    e.kind(),
                    ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset
                ) => {}
            _ => log::error!("Unknown connection error: {e:?}"),
        }
    }

    stream.shutdown().await.ok();

    #[cfg(debug_assertions)]
    log::debug!("Session closed: {session:?}");

    session.cleanup().await;
}

async fn handle_connection(
    stream: &mut TcpStream,
    session: SessionRef,
    mut rx: UnboundedReceiver<Value>,
) -> Result<()> {
    let mut shutdown_rx = shutdown::listen();
    let (mut read, mut writer) = stream.split();
    let mut reader = BufReader::new(&mut read);
    let mut writer = BufWriter::new(&mut writer);

    // Handle both reading and writing in the same task
    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.recv() => {
                return Ok(());
            }

            // Handle incoming messages from the channel
            Some(value) = rx.recv() => {
                #[cfg(debug_assertions)]
                log::debug!("Received message from client: {value:?}");

                session.respond(&value, &mut writer).await?;
            }

             // Handle incoming requests from the client
            result = Resp::try_parse(&mut reader, session.proto()) => {
                match result?.await? {
                    Some(Value::Multi(args)) => {
                        let mut args = Args::new(&args);

                        #[cfg(debug_assertions)]
                        log::debug!("Received command: {args:?} from session: {session:?}");

                        COMMANDS.handle(&mut writer, &mut args, &session).await?;
                    }
                    None => {
                        #[cfg(debug_assertions)]
                        log::debug!("Received empty request from session or stream closed: {session:?}");
                        return Ok(());
                    }
                    _ => {
                        session
                            .respond(&value_error!("Invalid request"), &mut writer)
                            .await?;
                    }
                }
            }
        }

        writer.flush().await?;
    }
}
