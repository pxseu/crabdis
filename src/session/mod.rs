use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::prelude::*;

pub mod state;

#[derive(Clone)]
pub struct Session {
    pub id: u64,
    pub state: state::StateRef,
    pub proto_version: Arc<RwLock<u8>>,
    pub tx: Arc<RwLock<Option<mpsc::Sender<Value>>>>,
}

impl Session {
    pub fn new(id: u64, state: StateRef) -> Arc<Self> {
        Arc::new(Self {
            id,
            state,
            // default to RESP2 protocol, can be changed via HELLO command
            proto_version: Arc::new(RwLock::new(2)),
            tx: Arc::new(RwLock::new(None)),
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
        writer: &mut WriteHalf<'_>,
    ) -> Result<()> {
        match self.get_proto_version().await {
            2 => response.to_resp2(writer).await,
            3 => response.to_resp3(writer).await,
            _ => unreachable!("Invalid protocol version"),
        }
    }

    pub async fn set_sender(&self, sender: mpsc::Sender<Value>) {
        *self.tx.write().await = Some(sender);
    }

    pub async fn send_versioned(&self, value: Value) -> Result<()> {
        if let Some(tx) = &*self.tx.read().await {
            tx.send(value).await.map_err(|e| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
        }
        Ok(())
    }

    pub async fn cleanup(&self) {
        self.state.remove_session(self.id).await;
    }
}

pub type SessionRef = Arc<Session>;
