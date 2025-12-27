use crate::prelude::*;

pub struct MSet;

#[async_trait]
impl CommandTrait for MSet {
    fn name(&self) -> &str {
        "MSET"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() < 2 || !args.len().is_multiple_of(2) {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;

        while let Some(key) = args.next() {
            match key {
                Value::String(k) => {
                    // safe to unwrap because we checked the length of the args
                    store.insert(k.clone(), args.next_owned().unwrap());
                }

                _ => {
                    return session
                        .versioned_response(&value_error!("Invalid key"), writer)
                        .await;
                }
            }
        }

        session.versioned_response(&Value::Ok, writer).await
    }
}
