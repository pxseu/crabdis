use crate::prelude::*;

pub struct Select;

#[async_trait]
impl CommandTrait for Select {
    fn name(&self) -> &'static str {
        "SELECT"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Select the Redis logical database having the specified zero-based numeric index",
            complexity: "O(1) for each element returned",
            since: "1.0.0",
        }
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        session.respond(&Value::Ok, writer).await
    }
}
