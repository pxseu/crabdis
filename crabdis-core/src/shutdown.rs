use std::fmt::{self, Display};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use tokio::sync::Notify;

use crate::prelude::*;

static SHUTDOWN_TX: OnceLock<Arc<ShutdownHandler>> = OnceLock::new();

struct ShutdownHandler {
    signal: AtomicU8,
    notify: Notify,
}

impl ShutdownHandler {
    fn new() -> Self {
        Self {
            signal: AtomicU8::new(Signal::NONE),
            notify: Notify::new(),
        }
    }

    fn signal(&self, s: Signal) {
        // Keep the first signal that arrives.
        let _ = self.signal.compare_exchange(
            Signal::NONE,
            s as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        self.notify.notify_waiters();
    }

    async fn wait_for_exit(&self) -> Signal {
        let signal_num = loop {
            let future = self.notify.notified();
            let signal = self.signal.load(Ordering::Acquire);

            if signal != Signal::NONE {
                break signal;
            }

            future.await;
        };

        Signal::from_u8(signal_num).unwrap_or_default()
    }
}

impl Default for ShutdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub enum Signal {
    #[default]
    Terminate = 1,
    Interrupt,
    Hangup,
}

impl Signal {
    /// Used to identify an empty atomic value for the signal
    const NONE: u8 = 0;

    const fn from_u8(s: u8) -> Option<Self> {
        Some(match s {
            1 => Self::Terminate,
            2 => Self::Interrupt,
            3 => Self::Hangup,
            _ => return None,
        })
    }
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

fn inner_shutdown() -> &'static ShutdownHandler {
    SHUTDOWN_TX.get_or_init(|| {
        let handler = Arc::new(ShutdownHandler::default());
        let h = handler.clone();

        tokio::spawn(async move {
            h.signal(wait_for_signal().await.expect("Could not bind a listener"));
        });

        handler
    })
}

/// Creates a new signal listener.
#[must_use]
pub async fn listen() -> Signal {
    inner_shutdown().wait_for_exit().await
}

/// Send a shutdown signal
pub fn send_shutdown() {
    inner_shutdown().signal(Signal::default());
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
