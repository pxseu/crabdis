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
        _session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        match args.pop_front() {
            Some(s) => s,
            _ => Value::Pong,
        }
        .to_resp2(writer)
        .await
    }
}
