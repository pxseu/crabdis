use glob::Pattern;

use crate::prelude::*;

pub struct Get;

#[async_trait]
impl SubcommandTrait for Get {
    fn name(&self) -> &'static str {
        "GET"
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: 3,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Gets the value of configuration parameters.",
            complexity: "O(N) where N is the number of configuration entries",
            since: "0.1.36",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(
                    &value_error!("ERR wrong number of arguments for 'CONFIG GET' command"),
                    writer,
                )
                .await;
        }

        let Some(pattern_str) = args.next_string() else {
            return session
                .respond(&value_error!("Invalid pattern"), writer)
                .await;
        };
        let pattern = Pattern::new(&pattern_str.to_lowercase())?;

        let mut response = HashMap::new();

        if pattern.matches("dir") {
            let dir = session.state.rdb_config.dir.read().await.clone();
            response.insert(
                Value::String("dir".into()),
                Value::String(dir.to_string_lossy().into_owned().into()),
            );
        }

        if pattern.matches("dbfilename") {
            let dbfilename = session.state.rdb_config.dbfilename.read().await.clone();

            response.insert(
                Value::String("dbfilename".into()),
                Value::String(dbfilename.into()),
            );
        }

        if pattern.matches("save") {
            let save_points = session.state.rdb_config.save_points.read().await.clone();
            let enabled = session.state.rdb_config.is_enabled();

            let save_value = if !enabled || save_points.is_empty() {
                String::new()
            } else {
                save_points
                    .iter()
                    .map(|sp| format!("{} {}", sp.seconds, sp.changes))
                    .collect::<Vec<_>>()
                    .join(" ")
            };

            response.insert(
                Value::String("save".into()),
                Value::String(save_value.into()),
            );
        }

        // TODO: Implement appendonly support
        if pattern.matches("appendonly") {
            response.insert(
                Value::String("appendonly".into()),
                Value::String("no".into()),
            );
        }

        session.respond(&Value::Map(response), writer).await
    }
}
