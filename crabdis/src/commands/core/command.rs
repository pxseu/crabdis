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
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            // Return command info as a map
            let commands = session.state.handler.commands.read().await;
            let mut map = HashMap::new();

            for (name, _) in commands.iter() {
                let cmd_info = vec![
                    Value::String(name.clone().into()),       // name
                    Value::Integer(-1),                       // arity (negative means variable)
                    Value::Multi(vec![Value::Nil; 0].into()), // flags
                    Value::Integer(0),                        // first key
                    Value::Integer(0),                        // last key
                    Value::Integer(0),                        // step
                ];
                map.insert(
                    Value::String(name.clone().into()),
                    Value::Multi(cmd_info.into()),
                );
            }

            return session.respond(&Value::Map(map), writer).await;
        }

        match args.next_string() {
            Some(subcommand) => {
                match subcommand.to_uppercase().as_str() {
                    "DOCS" => {
                        let commands = session.state.handler.commands.read().await;
                        let mut map = HashMap::new();

                        for (name, _) in commands.iter() {
                            let cmd_info = vec![
                                Value::String("Simple command".into()),   // summary
                                Value::String("O(1)".into()),             // complexity
                                Value::String("1.0.0".into()),            // since
                                Value::Multi(vec![Value::Nil; 0].into()), // arguments
                            ];
                            map.insert(
                                Value::String(name.clone().into()),
                                Value::Multi(cmd_info.into()),
                            );
                        }

                        session.respond(&Value::Map(map), writer).await
                    }
                    _ => {
                        session
                            .respond(&value_error!("Unknown subcommand"), writer)
                            .await
                    }
                }
            }
            _ => {
                session
                    .respond(&value_error!("Invalid subcommand"), writer)
                    .await
            }
        }
    }
}
