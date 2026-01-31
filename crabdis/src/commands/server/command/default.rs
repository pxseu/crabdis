use crate::commands::COMMANDS;
use crate::prelude::*;

/// Default handler for `COMMAND` (no subcommand).
///
/// Returns command info as a map.
pub struct Default;

#[async_trait]
impl Handler for Default {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let mut map = HashMap::new();

        for (name, cmd) in COMMANDS.iter() {
            let info = cmd.info();

            // Build basic command info array
            let mut cmd_info = vec![
                Value::String(name.into()),
                Value::Integer(info.arity),
                value_multi!(), // flags
                Value::Integer(info.first_key),
                Value::Integer(info.last_key),
                Value::Integer(info.step),
                value_multi!(), // ACL categories
                value_multi!(), // tips
                value_multi!(), // key specs
            ];

            // Element 10: subcommands
            if let Some(subcmds) = cmd.subcommands() {
                let sub_array: Vec<Value> = subcmds
                    .iter()
                    .map(|(sub_name, sub)| {
                        let sub_info = sub.info();
                        Value::Multi(
                            vec![
                                Value::String(sub_name.into()),
                                Value::Integer(sub_info.arity),
                                value_multi!(), // flags
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
                cmd_info.push(value_multi!());
            }

            map.insert(Value::String(name.into()), Value::Multi(cmd_info.into()));
        }

        session.respond(&Value::Map(map), writer).await
    }
}
