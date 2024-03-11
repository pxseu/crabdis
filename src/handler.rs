use tokio::io::AsyncWriteExt;

use crate::prelude::*;

pub async fn handle_client(
    mut stream: tokio::net::TcpStream,
    mut store: crate::storage::Store,
) -> Result<()> {
    let (mut read, mut writer) = stream.split();

    let mut reader = tokio::io::BufReader::new(&mut read);

    loop {
        let request = Value::from_resp(&mut reader).await?;

        #[cfg(debug_assertions)]
        log::debug!("Received request: {request:?}");

        let mut args = match request {
            Value::Multi(args) => args,
            _ => {
                writer
                    .write(&Value::Error("Invalid request".to_string()).bytes())
                    .await?;
                continue;
            }
        };

        let command = match args.pop_front() {
            Some(Value::String(command)) => command,
            _ => {
                writer
                    .write(&Value::Error("Invalid command".to_string()).bytes())
                    .await?;
                continue;
            }
        };

        let response = match command.as_str() {
            "PING" => match args.pop_front() {
                Some(message) => message,
                _ => Value::Pong,
            },

            "GET" => {
                if args.len() != 1 {
                    writer
                        .write(&Value::Error("Invalid number of arguments".to_string()).bytes())
                        .await?;
                    continue;
                }

                let key = match args.pop_front() {
                    Some(Value::String(key)) => key,
                    _ => {
                        writer
                            .write(&Value::Error("Invalid key".to_string()).bytes())
                            .await?;
                        continue;
                    }
                };

                store.get(&key).await
            }

            "MGET" => {
                let mut keys = vec![];

                for key in args {
                    if let Value::String(key) = key {
                        keys.push(key);
                    } else {
                        writer
                            .write(&Value::Error("Invalid key".to_string()).bytes())
                            .await?;
                        continue;
                    }
                }

                store.mget(&keys).await
            }

            "SET" => {
                if args.len() != 2 {
                    writer
                        .write(&Value::Error("Invalid number of arguments".to_string()).bytes())
                        .await?;
                    continue;
                }

                let key = match args.pop_front() {
                    Some(Value::String(key)) => key,
                    _ => {
                        writer
                            .write(&Value::Error("Invalid key".to_string()).bytes())
                            .await?;
                        continue;
                    }
                };

                let value = match args.pop_front() {
                    Some(value) => value,
                    _ => {
                        writer
                            .write(&Value::Error("Invalid value".to_string()).bytes())
                            .await?;
                        continue;
                    }
                };

                store.set(key, value).await;

                Value::Ok
            }
            "DEL" => {
                if args.len() != 1 {
                    writer
                        .write(&Value::Error("Invalid number of arguments".to_string()).bytes())
                        .await?;
                    continue;
                }

                let mut keys = vec![];

                for key in args {
                    if let Value::String(key) = key {
                        keys.push(key);
                    } else {
                        writer
                            .write(&Value::Error("Invalid key".to_string()).bytes())
                            .await?;
                        continue;
                    }
                }

                store.remove(&keys).await
            }
            "KEYS" => {
                if !args.is_empty() {
                    writer
                        .write(&Value::Error("Invalid number of arguments".to_string()).bytes())
                        .await?;
                    continue;
                }

                store.keys().await
            }
            "EXISTS" => {
                if args.len() != 1 {
                    writer
                        .write(&Value::Error("Invalid number    of arguments".to_string()).bytes())
                        .await?;

                    continue;
                }

                let key = match args.pop_front() {
                    Some(Value::String(key)) => key,
                    _ => {
                        writer
                            .write(&Value::Error("Invalid key".to_string()).bytes())
                            .await?;
                        continue;
                    }
                };

                let exists = store.exists(&key).await;

                Value::Integer(exists as i64)
            }

            "FLUSHDB" => {
                store.clear().await;

                Value::Ok
            }

            _ => Value::Error("Unknown command".to_string()),
        };

        #[cfg(debug_assertions)]
        log::debug!("Sending response: {response:?}");

        writer.write(&response.bytes()).await?;
    }
}
