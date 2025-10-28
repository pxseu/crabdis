use crate::prelude::*;

pub struct Publish;

#[async_trait]
impl CommandTrait for Publish {
    fn name(&self) -> &str {
        "PUBLISH"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 2 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let channel = match args.pop_front() {
            Some(Value::String(channel)) => channel,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid channel"), writer)
                    .await;
            }
        };

        let message = match args.pop_front() {
            Some(message) => message,
            _ => {
                return session
                    .versioned_response(&value_error!("Missing message"), writer)
                    .await;
            }
        };

        let count = session.state.publish(&channel, message).await?;
        Value::Integer(count).to_resp2(writer).await
    }
}
