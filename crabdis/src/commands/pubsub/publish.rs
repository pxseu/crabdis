use crate::prelude::*;

pub struct Publish;

#[async_trait]
impl CommandTrait for Publish {
    fn name(&self) -> &'static str {
        "PUBLISH"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 3,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Posts a message to a channel",
            complexity: "O(N+M) where N is the number of clients subscribed to the channel and M is the number of clients subscribed to patterns that match the channel.",
            since: "0.1.34",
        }
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

        let Some(channel) = args.next_string() else {
            return session
                .respond(&value_error!("Invalid channel"), writer)
                .await;
        };

        let Some(message) = args.next_owned() else {
            return session
                .respond(&value_error!("Missing message"), writer)
                .await;
        };

        let count = session.state.publish(channel, message).await?;
        session.respond(&Value::Integer(count), writer).await
    }
}
