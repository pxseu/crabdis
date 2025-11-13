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
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let mut channels = Vec::new();
        while let Some(arg) = args.pop_front() {
            match arg {
                Value::String(channel) => {
                    session.state.subscribe(&channel, session.clone()).await;
                    channels.push(channel);
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
