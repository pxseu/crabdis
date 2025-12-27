use crate::prelude::*;

pub struct Ping;

#[async_trait]
impl CommandTrait for Ping {
    fn name(&self) -> &str {
        "PING"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let response = match args.pop_front() {
            Some(s) => s,
            _ => Value::Pong,
        };
        session.versioned_response(&response, writer).await
    }
}
