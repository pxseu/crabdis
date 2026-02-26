use std::io::ErrorKind;
use std::net::SocketAddr;

use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::commands::COMMANDS;
use crate::prelude::*;
use crate::session::Session;

pub async fn handle_client(mut stream: TcpStream, state: StateRef, socket: SocketAddr) {
    // Disable Nagle's algorithm to ensure immediate delivery of data
    if let Err(e) = stream.set_nodelay(true) {
        log::error!("Failed to set TCP_NODELAY: {e}");
        return;
    }

    let (tx, rx) = mpsc::unbounded_channel();
    let session_id = state.get_next_session_id().await;
    let session = Session::new(session_id, socket, state.clone(), tx);
    state.add_session(session.clone()).await;

    #[cfg(debug_assertions)]
    log::debug!("Accepted connection from {socket} for session: {session:?}");

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

    #[cfg(debug_assertions)]
    log::debug!("Connection from {socket} closed");
}

async fn handle_connection(
    stream: &mut TcpStream,
    session: SessionRef,
    mut rx: mpsc::UnboundedReceiver<Value>,
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
