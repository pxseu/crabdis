use crate::commands::all_commands;
use crate::prelude::*;

pub struct Command;

#[async_trait]
impl CommandTrait for Command {
    fn name(&self) -> &'static str {
        "COMMAND"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns details about Redis commands",
            complexity: "O(N) where N is the number of commands",
            since: "0.1.34",
        }
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            // Return command info as a map
            let mut map = HashMap::new();

            for (name, t) in all_commands() {
                let info = t.info();
                let cmd_info = vec![
                    Value::String(name.as_str().into()),
                    Value::Integer(info.arity),
                    Value::Multi(Vec::new().into()),
                    Value::Integer(info.first_key),
                    Value::Integer(info.last_key),
                    Value::Integer(info.step),
                ];
                map.insert(
                    Value::String(name.clone().into()),
                    Value::Multi(cmd_info.into()),
                );
            }

            return session.respond(&Value::Map(map), writer).await;
        }

        match args.next_string() {
            Some(subcommand) => match subcommand.to_uppercase().as_str() {
                "DOCS" => {
                    let mut map = HashMap::new();

                    for (name, t) in all_commands() {
                        let info = t.info();

                        let cmd_info = vec![
                            Value::String(info.summary.into()),
                            Value::String(info.complexity.into()),
                            Value::String(info.since.into()),
                            Value::Multi(Vec::new().into()),
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
            },
            _ => {
                session
                    .respond(&value_error!("Invalid subcommand"), writer)
                    .await
            }
        }
    }
}
