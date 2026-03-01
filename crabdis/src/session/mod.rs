use std::net::SocketAddr;

use tokio::sync::{RwLock, mpsc};
use tokio::time::Instant;

use crate::prelude::*;

pub mod auth;
pub mod state;

pub struct Session {
    pub id: u64,
    // private since are rwlocked and accessed via methods
    name: RwLock<Option<Arc<str>>>,
    proto_version: AtomicVersion,
    authenticated: AtomicBool,
    pub age: Instant,
    pub socket_addr: SocketAddr,
    pub state: state::StateRef,
    pub tx: mpsc::UnboundedSender<Value>,
    // Static refers to the Counter itself, defined above
    _counter_guard: CounterGuard<'static>,
}

#[cfg(debug_assertions)]
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field(
                "name",
                &loop {
                    if let Ok(name) = self.name.try_read() {
                        break name;
                    }
                },
            )
            .field("proto", &self.proto_version)
            .field("authenticated", &self.authenticated)
            .field("age", &Instant::now().duration_since(self.age))
            .field("socket_addr", &self.socket_addr)
            .finish_non_exhaustive()
    }
}

impl Session {
    pub fn new(
        id: u64,
        socket: SocketAddr,
        state: StateRef,
        tx: mpsc::UnboundedSender<Value>,
        guard: CounterGuard<'static>,
    ) -> Arc<Self> {
        Arc::new(Self {
            authenticated: AtomicBool::new(!state.auth.has_auth()),
            id,
            state,
            socket_addr: socket,
            age: Instant::now(),
            name: RwLock::new(None),
            // default to RESP2 protocol, can be changed via HELLO command
            proto_version: AtomicVersion::new(),
            tx,
            _counter_guard: guard,
        })
    }

    #[inline]
    pub fn proto(&self) -> Version {
        self.proto_version.get()
    }

    pub fn set_proto(&self, proto: Version) {
        self.proto_version.set(proto);
    }

    #[inline]
    pub fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::Relaxed)
    }

    pub fn set_authenticated(&self, authenticated: bool) {
        self.authenticated.store(authenticated, Ordering::Relaxed);
    }

    pub async fn respond(
        &self,
        response: &Value,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
    ) -> Result<()> {
        #[cfg(debug_assertions)]
        log::debug!("Writing response to client: {response:?}");

        Resp::write(response, writer, self.proto()).await?;

        Ok(())
    }

    pub fn send(&self, value: Value) -> Result<()> {
        self.tx
            .send(value)
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))
    }

    pub async fn set_name(&self, name: Arc<str>) {
        *self.name.write().await = Some(name);
    }

    pub async fn name(&self) -> Option<Arc<str>> {
        self.name.read().await.clone()
    }

    pub async fn cleanup(&self) {
        self.state.remove_session(self.id).await;
    }
}

pub type SessionRef = Arc<Session>;
