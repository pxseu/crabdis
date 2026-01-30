use crate::commands::COMMANDS;
use crate::prelude::*;

/// Default handler for `COMMAND` (no subcommand).
///
/// Returns command info as a map.
pub struct Default;

static EMPTY_ARC_SLICE: LazyLock<Arc<[Value]>> = LazyLock::new(|| Arc::from([]));

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
                Value::Multi(EMPTY_ARC_SLICE.clone()), // flags
                Value::Integer(info.first_key),
                Value::Integer(info.last_key),
                Value::Integer(info.step),
                Value::Multi(EMPTY_ARC_SLICE.clone()), // ACL categories
                Value::Multi(EMPTY_ARC_SLICE.clone()), // tips
                Value::Multi(EMPTY_ARC_SLICE.clone()), // key specs
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
                                Value::Multi(EMPTY_ARC_SLICE.clone()), // flags
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
                cmd_info.push(Value::Multi(EMPTY_ARC_SLICE.clone()));
            }

            map.insert(Value::String(name.into()), Value::Multi(cmd_info.into()));
        }

        session.respond(&Value::Map(map), writer).await
    }
}
