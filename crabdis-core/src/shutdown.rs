use std::fmt::{self, Display};
use std::sync::OnceLock;

use tokio::sync::broadcast;

use crate::prelude::*;

pub type Receiver = broadcast::Receiver<Signal>;

static SHUTDOWN_TX: OnceLock<broadcast::Sender<Signal>> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum Signal {
    Terminate,
    Interrupt,
    Hangup,
}

impl Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminate => write!(f, "SIGTERM"),
            Self::Interrupt => write!(f, "SIGINT"),
            Self::Hangup => write!(f, "SIGHUP"),
        }
    }
}

/// Creates a new signal listener.
///
/// # Panics
///
/// Panics if the signal listener cannot be created.
pub fn listen() -> Receiver {
    let tx = SHUTDOWN_TX.get_or_init(|| {
        let (shutdown_tx, _) = broadcast::channel(1);

        // Spawn signal handler task
        tokio::spawn(async move {
            let result = wait_for_signal().await.expect("Failed to wait for signal");

            // Send shutdown signal, there could technically be a race condition here if the
            // sender is not saved yet, but it's unlikely to happen.
            if let Some(tx) = SHUTDOWN_TX.get() {
                let _ = tx.send(result);
            }
        });

        shutdown_tx
    });

    tx.subscribe()
}

#[cfg(unix)]
async fn wait_for_signal() -> Result<Signal> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sighup = signal(SignalKind::hangup())?;

    tokio::select! {
        _ = sigterm.recv() => Ok(Signal::Terminate),
        _ = sigint.recv() => Ok(Signal::Interrupt),
        _ = sighup.recv() => Ok(Signal::Hangup),
    }
}

#[cfg(windows)]
async fn wait_for_signal() -> Result<Signal> {
    use tokio::signal::windows;

    let mut ctrlc = windows::ctrl_c()?;
    let mut ctrlbreak = windows::ctrl_break()?;
    let mut ctrlclose = windows::ctrl_close()?;

    tokio::select! {
        _ = ctrlc.recv() => Ok(Signal::Interrupt),
        _ = ctrlbreak.recv() => Ok(Signal::Terminate),
        _ = ctrlclose.recv() => Ok(Signal::Hangup),
    }
}
