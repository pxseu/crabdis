use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use tokio::sync::{RwLock, mpsc};

use crate::prelude::*;

pub mod state;

pub struct Session {
    pub id: u64,
    // private since are rwlocked and accessed via methods
    name: RwLock<Option<Arc<str>>>,
    proto_version: AtomicU8,
    pub state: state::StateRef,
    pub tx: mpsc::UnboundedSender<Value>,
}

impl Session {
    pub fn new(id: u64, state: StateRef, tx: mpsc::UnboundedSender<Value>) -> Arc<Self> {
        Arc::new(Self {
            id,
            state,
            name: RwLock::new(None),
            // default to RESP2 protocol, can be changed via HELLO command
            proto_version: AtomicU8::new(2),
            tx,
        })
    }

    pub fn get_proto_version(&self) -> u8 {
        self.proto_version.load(Ordering::Relaxed)
    }

    pub fn set_proto_version(&self, version: u8) {
        self.proto_version.store(version, Ordering::Relaxed);
    }

    pub async fn versioned_response(
        &self,
        response: &Value,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
    ) -> Result<()> {
        #[cfg(debug_assertions)]
        log::debug!("Writing response to client: {:?}", response);

        match self.get_proto_version() {
            2 => Resp::to2(response, writer).await.map_err(Error::from),
            3 => Resp::to3(response, writer).await.map_err(Error::from),
            _ => unreachable!("Invalid protocol version"),
        }
    }

    pub fn send(&self, value: Value) -> Result<()> {
        self.tx
            .send(value)
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))
    }

    pub async fn set_name(&self, name: Arc<str>) {
        *self.name.write().await = Some(name);
    }

    pub async fn get_name(&self) -> Option<Arc<str>> {
        self.name.read().await.clone()
    }

    pub async fn cleanup(&self) {
        self.state.remove_session(self.id).await;
    }
}

pub type SessionRef = Arc<Session>;
