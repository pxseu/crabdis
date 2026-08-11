use std::path::PathBuf;

use crate::prelude::*;
use crate::storage::rdb;

#[derive(Subcommand)]
#[command(
    arity = -4,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Sets configuration parameters.",
    complexity = "O(N) where N is the number of parameters to set",
    since = "0.1.36",
)]
pub struct Set;

#[async_trait]
impl Handler for Set {
    #[allow(clippy::too_many_lines)]
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() < 2 {
            return session
                .respond(
                    &value_error!("ERR wrong number of arguments for 'CONFIG SET' command"),
                    writer,
                )
                .await;
        }

        let Some(option) = args.next_string_owned() else {
            return session
                .respond(
                    &value_error!("ERR wrong number of arguments for 'CONFIG SET' command"),
                    writer,
                )
                .await;
        };

        match option.to_ascii_lowercase().as_str() {
            "dir" => {
                let Some(value) = args.next_string_owned() else {
                    return session
                        .respond(&value_error!("ERR invalid value"), writer)
                        .await;
                };

                if !args.is_empty() {
                    return session
                        .respond(
                            &value_error!("ERR wrong number of arguments for 'CONFIG SET' command"),
                            writer,
                        )
                        .await;
                }

                *session.state.rdb_config.dir.write().await = PathBuf::from(value.as_ref());
                session.respond(&Value::Ok, writer).await
            }
            "dbfilename" => {
                let Some(value) = args.next_string_owned() else {
                    return session
                        .respond(&value_error!("ERR invalid value"), writer)
                        .await;
                };

                if !args.is_empty() {
                    return session
                        .respond(
                            &value_error!("ERR wrong number of arguments for 'CONFIG SET' command"),
                            writer,
                        )
                        .await;
                }

                *session.state.rdb_config.dbfilename.write().await = value.to_string();
                session.respond(&Value::Ok, writer).await
            }
            "save" => {
                let mut raw_values = Vec::new();
                for value in args {
                    match value {
                        Value::String(value) => raw_values.push(value.to_string()),
                        Value::Integer(value) => raw_values.push(value.to_string()),
                        _ => {
                            return session
                                .respond(&value_error!("ERR invalid save point"), writer)
                                .await;
                        }
                    }
                }

                if raw_values.is_empty() {
                    return session
                        .respond(
                            &value_error!("ERR wrong number of arguments for 'CONFIG SET' command"),
                            writer,
                        )
                        .await;
                }

                if raw_values.len() == 1 && raw_values[0].is_empty() {
                    session.state.rdb_config.save_points.write().await.clear();
                    session
                        .state
                        .rdb_config
                        .enabled
                        .store(false, Ordering::Relaxed);
                    return session.respond(&Value::Ok, writer).await;
                }

                let mut tokens = Vec::new();
                for value in raw_values {
                    tokens.extend(value.split_whitespace().map(str::to_string));
                }

                if tokens.len() % 2 != 0 || tokens.is_empty() {
                    return session
                        .respond(&value_error!("ERR invalid save point"), writer)
                        .await;
                }

                let mut save_points = Vec::new();
                for pair in tokens.chunks(2) {
                    let Ok(seconds) = pair[0].parse::<u64>() else {
                        return session
                            .respond(&value_error!("ERR invalid save point"), writer)
                            .await;
                    };
                    let Ok(changes) = pair[1].parse::<u64>() else {
                        return session
                            .respond(&value_error!("ERR invalid save point"), writer)
                            .await;
                    };

                    save_points.push(SavePoint::new(seconds, changes));
                }

                *session.state.rdb_config.save_points.write().await = save_points;
                session.state.rdb_config.set_enabled(true);

                if session
                    .state
                    .auto_save_started
                    .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    rdb::spawn_auto_save_task(session.state.clone());
                }

                session.respond(&Value::Ok, writer).await
            }
            _ => {
                session
                    .respond(&value_error!("ERR unknown option '{option}'"), writer)
                    .await
            }
        }
    }
}
