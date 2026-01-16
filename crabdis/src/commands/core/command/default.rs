use crate::commands::COMMANDS;
use crate::prelude::*;

/// Default handler for `COMMAND` (no subcommand).
///
/// Returns command info as a map.
pub struct Default;

#[async_trait]
impl SubcommandTrait for Default {
    fn name(&self) -> &'static str {
        ""
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: 1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns details about all Redis commands.",
            complexity: "O(N) where N is the number of commands",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let mut map = HashMap::new();

        for (name, cmd) in COMMANDS.iter() {
            let info = cmd.info();

            // Build basic command info array
            let mut cmd_info = vec![
                Value::String(name.clone()),
                Value::Integer(info.arity),
                Value::Multi(Vec::new().into()), // flags
                Value::Integer(info.first_key),
                Value::Integer(info.last_key),
                Value::Integer(info.step),
                Value::Multi(Vec::new().into()), // ACL categories
                Value::Multi(Vec::new().into()), // tips
                Value::Multi(Vec::new().into()), // key specs
            ];

            // Element 10: subcommands
            if let Some(subcmds) = cmd.subcommands() {
                let sub_array: Vec<Value> = subcmds
                    .iter()
                    .map(|(sub_name, sub)| {
                        let sub_info = sub.info();
                        Value::Multi(
                            vec![
                                Value::String(sub_name.clone()),
                                Value::Integer(sub_info.arity),
                                Value::Multi(Vec::new().into()), // flags
                                Value::Integer(sub_info.first_key),
                                Value::Integer(sub_info.last_key),
                                Value::Integer(sub_info.step),
                            ]
                            .into(),
                        )
                    })
                    .collect();
                cmd_info.push(Value::Multi(sub_array.into()));
            } else {
                cmd_info.push(Value::Multi(Vec::new().into()));
            }

            map.insert(Value::String(name.clone()), Value::Multi(cmd_info.into()));
        }

        session.respond(&Value::Map(map), writer).await
    }
}
