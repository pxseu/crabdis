use crate::prelude::*;

pub struct Subscribe;

#[async_trait]
impl CommandTrait for Subscribe {
    fn name(&self) -> &str {
        "SUBSCRIBE"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
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
                    session
                        .state
                        .subscribe(channel.clone(), session.clone())
                        .await;
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
            let mut response = VecDeque::new();
            response.push_back(Value::String("subscribe".to_string()));
            response.push_back(Value::String(channel));
            response.push_back(Value::Integer(1)); // Number of subscriptions

            session
                .versioned_response(&Value::Multi(response), writer)
                .await?;
        }

        Ok(())
    }
}
