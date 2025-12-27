use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use crate::prelude::*;

pub mod state;

pub struct Session {
    pub id: u64,
    // private since are rwlocked and accessed via methods
    name: RwLock<Option<Arc<str>>>,
    proto_version: RwLock<u8>,
    pub state: state::StateRef,
    pub tx: RwLock<Option<mpsc::UnboundedSender<Value>>>,
}

impl Session {
    pub fn new(id: u64, state: StateRef) -> Arc<Self> {
        Arc::new(Self {
            id,
            state,
            name: RwLock::new(None),
            // default to RESP2 protocol, can be changed via HELLO command
            proto_version: RwLock::new(2),
            tx: RwLock::new(None),
        })
    }

    pub async fn get_proto_version(&self) -> u8 {
        *self.proto_version.read().await
    }

    pub async fn set_proto_version(&self, version: u8) {
        *self.proto_version.write().await = version;
    }

    pub async fn versioned_response(
        &self,
        response: &Value,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
    ) -> Result<()> {
        #[cfg(debug_assertions)]
        log::debug!("Writing response to client: {:?}", response);

        match self.get_proto_version().await {
            2 => Resp::to2(response, writer).await.map_err(Error::from),
            3 => Resp::to3(response, writer).await.map_err(Error::from),
            _ => unreachable!("Invalid protocol version"),
        }
    }

    pub async fn set_sender(&self, sender: mpsc::UnboundedSender<Value>) {
        *self.tx.write().await = Some(sender);
    }

    pub async fn send_versioned(&self, value: Value) -> Result<()> {
        if let Some(tx) = &*self.tx.read().await {
            tx.send(value)
                .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        }
        Ok(())
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
