use crate::commands::COMMANDS;
use crate::prelude::*;

pub struct Docs;

#[async_trait]
impl SubcommandTrait for Docs {
    fn name(&self) -> &'static str {
        "DOCS"
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: -2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns documentary information about commands.",
            complexity: "O(N) where N is the number of commands to look up",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let mut map = HashMap::new();

        for (name, cmd) in COMMANDS.iter() {
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
                    sub_docs.insert(Value::String(sub_name.as_str().into()), Value::Map(sub_doc));
                }
                cmd_doc.insert(Value::String("subcommands".into()), Value::Map(sub_docs));
            }

            map.insert(Value::String(name.as_str().into()), Value::Map(cmd_doc));
        }

        session.respond(&Value::Map(map), writer).await
    }
}
