use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

pub mod state;

pub struct Session {
    pub state: state::StateRef,
    pub proto_version: RwLock<u8>,
    // TODO: auth maybe?
}

impl Session {
    pub fn new(state: StateRef) -> Arc<Self> {
        Arc::new(Self {
            state,
            // default to RESP2 protocol, can be changed via HELLO command
            proto_version: RwLock::new(2),
        })
    }

    pub async fn get_proto_version(&self) -> i64 {
        *self.proto_version.read().await as i64
    }

    pub async fn set_proto_version(&self, version: i64) {
        *self.proto_version.write().await = version as u8;
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
}

pub type SessionRef = Arc<Session>;
