use crate::commands::COMMANDS;
use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = -2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns documentary information about commands.",
    complexity = "O(N) where N is the number of commands to look up",
    since = "0.1.34",
)]
pub struct Docs;

#[async_trait]
impl Handler for Docs {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let map: HashMap<Value, Value> = if args.is_empty() {
            COMMANDS.values().map(build_command_doc).collect()
        } else {
            args.iter()
                .filter_map(|arg| match arg {
                    Value::String(name) => Some(COMMANDS.get(name.as_ref()).map_or_else(
                        || (Value::String(name.clone()), Value::Nil),
                        build_command_doc,
                    )),
                    _ => None,
                })
                .collect()
        };

        session.respond(&Value::Map(map), writer).await
    }
}

fn build_command_doc(cmd: &(dyn CommandTrait + Send + Sync)) -> (Value, Value) {
    let info = cmd.info();

    let mut cmd_doc = HashMap::new();
    cmd_doc.insert(
        Value::String("summary".into()),
        Value::String(info.summary.into()),
    );
    cmd_doc.insert(
        Value::String("complexity".into()),
        Value::String(info.complexity.into()),
    );
    cmd_doc.insert(
        Value::String("since".into()),
        Value::String(info.since.into()),
    );

    // Add subcommand docs if present
    if let Some(subcmds) = cmd.subcommands() {
        let mut sub_docs = HashMap::new();
        for (sub_name, sub) in subcmds.iter() {
            let sub_info = sub.info();
            let mut sub_doc = HashMap::new();
            sub_doc.insert(
                Value::String("summary".into()),
                Value::String(sub_info.summary.into()),
            );
            sub_doc.insert(
                Value::String("complexity".into()),
                Value::String(sub_info.complexity.into()),
            );
            sub_doc.insert(
                Value::String("since".into()),
                Value::String(sub_info.since.into()),
            );
            sub_docs.insert(Value::String(sub_name.into()), Value::Map(sub_doc));
        }
        cmd_doc.insert(Value::String("subcommands".into()), Value::Map(sub_docs));
    }

    (Value::String(cmd.name().into()), Value::Map(cmd_doc))
}
