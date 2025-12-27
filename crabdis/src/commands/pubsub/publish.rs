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
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let channel = match args.next() {
            Some(Value::String(channel)) => channel,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid channel"), writer)
                    .await;
            }
        };

        let message = match args.next_owned() {
            Some(message) => message,
            _ => {
                return session
                    .versioned_response(&value_error!("Missing message"), writer)
                    .await;
            }
        };

        let count = session.state.publish(channel, message).await?;
        session
            .versioned_response(&Value::Integer(count), writer)
            .await
    }
}
