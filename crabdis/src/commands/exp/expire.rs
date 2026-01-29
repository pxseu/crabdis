use std::hint::unreachable_unchecked;

use crabdis_core::error::Error as CoreError;
use crabdis_core::store::error::StoreError;
use tokio::time::{Duration, Instant};

use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -3,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Set a key's time to live in seconds",
    complexity = "O(1)",
    since = "0.1.34",
)]
pub struct Expire;

#[async_trait]
impl Handler for Expire {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let seconds = match args.next() {
            Some(Value::Integer(seconds)) => *seconds,
            Some(Value::String(seconds)) => seconds.parse::<i64>().unwrap_or(-1),
            Some(_) => {
                return session
                    .respond(&value_error!("Invalid seconds"), writer)
                    .await;
            }
            None => {
                return session
                    .respond(&value_error!("Missing seconds"), writer)
                    .await;
            }
        };

        if seconds <= 0 {
            return session
                .respond(&value_error!("Invalid seconds"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;
        let mut expire_keys = session.state.expire_keys.write().await;

        let value = match store.get_unexpired_mut(&key) {
            Err(CoreError::Store(e)) => {
                if matches!(e, StoreError::Expired) {
                    store.remove(&key);
                    expire_keys.remove(&key);
                }

                return session.respond(&Value::Integer(0), writer).await;
            }
            Err(_) => unsafe { unreachable_unchecked() },
            Ok(v) => v,
        };

        value.set_expire(Instant::now() + Duration::from_secs(seconds as u64));
        expire_keys.insert(key);

        session.state.notify_change();

        session.respond(&Value::Integer(1), writer).await
    }
}
