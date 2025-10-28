use crate::prelude::*;

pub struct Command;

#[async_trait]
impl CommandTrait for Command {
    fn name(&self) -> &str {
        "COMMAND"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            // Return command info as a map
            let commands = session.state.handler.commands.read().await;
            let mut map = HashMap::new();

            for (name, _) in commands.iter() {
                let mut cmd_info = Vec::new();
                cmd_info.push(Value::String(name.clone().into())); // name
                cmd_info.push(Value::Integer(-1)); // arity (negative means variable)
                cmd_info.push(Value::Multi(vec![Value::Nil; 0].into())); // flags
                cmd_info.push(Value::Integer(0)); // first key
                cmd_info.push(Value::Integer(0)); // last key
                cmd_info.push(Value::Integer(0)); // step
                map.insert(
                    Value::String(name.clone().into()),
                    Value::Multi(cmd_info.into()),
                );
            }

            return session.versioned_response(&Value::Map(map), writer).await;
        }

        match args.pop_front() {
            Some(Value::String(subcommand)) => {
                match subcommand.to_uppercase().as_str() {
                    "DOCS" => {
                        let commands = session.state.handler.commands.read().await;
                        let mut map = HashMap::new();

                        for (name, _) in commands.iter() {
                            let mut cmd_info = Vec::new();
                            cmd_info.push(Value::String("Simple command".into())); // summary
                            cmd_info.push(Value::String("O(1)".into())); // complexity
                            cmd_info.push(Value::String("1.0.0".into())); // since
                            cmd_info.push(Value::Multi(vec![Value::Nil; 0].into())); // arguments
                            map.insert(
                                Value::String(name.clone().into()),
                                Value::Multi(cmd_info.into()),
                            );
                        }

                        session.versioned_response(&Value::Map(map), writer).await
                    }
                    _ => {
                        session
                            .versioned_response(&value_error!("Unknown subcommand"), writer)
                            .await
                    }
                }
            }
            _ => {
                session
                    .versioned_response(&value_error!("Invalid subcommand"), writer)
                    .await
            }
        }
    }
}
