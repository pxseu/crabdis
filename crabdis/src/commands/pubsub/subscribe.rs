use crate::prelude::*;

pub struct Subscribe;

#[async_trait]
impl CommandTrait for Subscribe {
    fn name(&self) -> &str {
        "SUBSCRIBE"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let mut channels = Vec::new();
        for arg in args.iter() {
            match arg {
                Value::String(channel) => {
                    session.state.subscribe(channel, session.clone()).await;
                    channels.push(channel.clone());
                }
                _ => {
                    return session
                        .versioned_response(&value_error!("Invalid channel"), writer)
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
                .versioned_response(&Value::Push(response.into()), writer)
                .await?;
        }

        Ok(())
    }
}
