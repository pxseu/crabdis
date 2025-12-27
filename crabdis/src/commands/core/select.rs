use crate::prelude::*;

pub struct Select;

#[async_trait]
impl CommandTrait for Select {
    fn name(&self) -> &str {
        "SELECT"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        session.versioned_response(&Value::Ok, writer).await
    }
}
