use crate::commands::COMMANDS;
use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = -2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns details about commands.",
    complexity = "O(N) where N is the number of commands to look up",
    since = "0.1.38"
)]
pub struct Info;

#[async_trait]
impl Handler for Info {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let map: HashMap<Value, Value> = if args.is_empty() {
            COMMANDS.values().map(build_command_info).collect()
        } else {
            args.iter()
                .filter_map(|arg| match arg {
                    Value::String(name) => Some(COMMANDS.get(name.as_ref()).map_or_else(
                        || (Value::String(name.clone()), Value::Nil),
                        build_command_info,
                    )),
                    _ => None,
                })
                .collect()
        };

        session.respond(&Value::Map(map), writer).await
    }
}

fn build_command_info(cmd: &(dyn CommandTrait + Send + Sync)) -> (Value, Value) {
    let name = Value::String(cmd.name().into());
    let info = cmd.info();

    // Build basic command info array
    let mut cmd_info = vec![
        name.clone(),
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

    (name, Value::Multi(cmd_info.into()))
}
