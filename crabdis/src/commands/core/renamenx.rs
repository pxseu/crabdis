use crate::prelude::*;

pub struct RenameNx;

#[async_trait]
impl CommandTrait for RenameNx {
    fn name(&self) -> &'static str {
        "RENAMENX"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session
                .respond(&value_error!("Invalid argument"), writer)
                .await;
        };

        let Some(new_key) = args.next_string_owned() else {
            return session
                .respond(&value_error!("Invalid argument"), writer)
                .await;
        };

        // check new
        let mut locked = session.state.store.write().await;

        if locked.contains_key(&new_key) {
            return session.respond(&Value::Nil, writer).await;
        }

        let Some((_, old_data)) = locked.remove_entry(&key) else {
            return session
                .respond(&value_error!("Key to be reanmed not found"), writer)
                .await;
        };

        locked.insert(new_key, old_data);

        session.state.notify_change();

        session.respond(&Value::Ok, writer).await
    }
}
