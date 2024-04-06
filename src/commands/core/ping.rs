use crate::prelude::*;

pub struct Ping;

#[async_trait]
impl CommandTrait for Ping {
    fn name(&self) -> &str {
        "PING"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        _session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let response = match args.pop_front() {
            Some(s) => s,
            _ => Value::Pong,
        };

        response.to_resp2(writer).await
    }
}
