use crate::prelude::*;

pub struct RenameNx;

#[async_trait]
impl CommandTrait for RenameNx {
    fn name(&self) -> &str {
        "RENAMENX"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let key = match args.pop_front() {
            Some(Value::String(s)) => s,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid argument"), writer)
                    .await;
            }
        };

        let new_key = match args.pop_front() {
            Some(Value::String(s)) => s,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid argument"), writer)
                    .await;
            }
        };

        // check new
        let mut locked = session.state.store.write().await;

        if locked.contains_key(&new_key) {
            return session.versioned_response(&Value::Nil, writer).await;
        }

        let Some((_, old_data)) = locked.remove_entry(&key) else {
            return session
                .versioned_response(&value_error!("Key to be reanmed not found"), writer)
                .await;
        };

        locked.insert(new_key, old_data);

        session.versioned_response(&Value::Ok, writer).await
    }
}
