use crate::prelude::*;

pub struct Subscribe;

#[async_trait]
impl CommandTrait for Subscribe {
    fn name(&self) -> &'static str {
        "SUBSCRIBE"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Subscribes to one or more channels",
            complexity: "O(N) where N is the number of channels to subscribe to",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.is_empty() {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let mut channels = Vec::new();
        for arg in args {
            match arg {
                Value::String(channel) => {
                    session.state.subscribe(channel, session.id).await;
                    channels.push(channel.clone());
                }
                _ => {
                    return session
                        .respond(&value_error!("Invalid channel"), writer)
                        .await;
                }
            }
        }

        // Send subscription confirmation for each channel
        for channel in channels {
            let response = vec![
                Value::String("subscribe".into()),
                Value::String(channel),
                Value::Integer(1), // Number of subscriptions
            ];

            session
                .respond(&Value::Push(response.into()), writer)
                .await?;
        }

        Ok(())
    }
}
