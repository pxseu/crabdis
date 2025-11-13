use crate::prelude::*;

pub struct Unsubscribe;

#[async_trait]
impl CommandTrait for Unsubscribe {
    fn name(&self) -> &str {
        "UNSUBSCRIBE"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        let mut channels = Vec::new();

        // If no channels specified, unsubscribe from all
        if args.is_empty() {
            let subs = session.state.subscriptions.read().await;
            channels = subs.keys().cloned().collect();
        } else {
            while let Some(arg) = args.pop_front() {
                match arg {
                    Value::String(channel) => channels.push(channel),
                    _ => {
                        return session
                            .versioned_response(&value_error!("Invalid channel"), writer)
                            .await;
                    }
                }
            }
        }

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
                .versioned_response(&Value::Multi(response.into()), writer)
                .await?;
        }

        Ok(())
    }
}
