use crate::prelude::*;

pub struct Unsubscribe;

#[async_trait]
impl CommandTrait for Unsubscribe {
    fn name(&self) -> &'static str {
        "UNSUBSCRIBE"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Unsubscribes from one or more channels",
            complexity: "O(N) where N is the number of channels to unsubscribe from",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        // If no channels specified, unsubscribe from all
        let channels = if args.is_empty() {
            let subs = session.state.subscriptions.read().await;
            subs.keys().cloned().collect()
        } else {
            let mut channels = Vec::new();
            for arg in args {
                match arg {
                    Value::String(channel) => channels.push(channel.clone()),
                    _ => {
                        return session
                            .respond(&value_error!("Invalid channel"), writer)
                            .await;
                    }
                }
            }
            channels
        };

        // Unsubscribe from each channel
        for channel in &channels {
            session.state.unsubscribe(channel, &session).await;
        }

        // Send unsubscription confirmation for each channel
        for channel in channels {
            let response = vec![
                Value::String("unsubscribe".into()),
                Value::String(channel),
                Value::Integer(0), // Number of remaining subscriptions
            ];

            session
                .respond(&Value::Multi(response.into()), writer)
                .await?;
        }

        Ok(())
    }
}
