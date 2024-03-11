pub use crate::prelude::*;

pub async fn handle_command(
    command: &str,
    args: &mut VecDeque<Value>,
    store: &mut Store,
) -> Result<Value> {
    let response = match command {
        "PING" => match args.pop_front() {
            Some(message) => message,
            _ => Value::Pong,
        },

        "GET" => {
            if args.len() != 1 {
                return Ok(Value::Error("Invalid number of arguments".to_string()));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(Value::Error("Invalid key".to_string()));
                }
            };

            store.get(&key).await
        }

        "MGET" => {
            let mut keys = vec![];

            for key in args {
                if let Value::String(key) = key {
                    keys.push(key.to_owned());
                } else {
                    return Ok(Value::Error("Invalid key".to_string()));
                }
            }

            store.mget(&keys).await
        }

        "SET" => {
            if args.len() != 2 {
                return Ok(Value::Error("Invalid number of arguments".to_string()));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(Value::Error("Invalid key".to_string()));
                }
            };

            let value = match args.pop_front() {
                Some(value) => value,
                _ => {
                    return Ok(Value::Error("Invalid value".to_string()));
                }
            };

            store.set(key, value).await;

            Value::Ok
        }

        "MSET" => {
            let mut data = HashMap::new();

            if args.len() % 2 != 0 {
                return Ok(Value::Error("Invalid number of arguments".to_string()));
            }

            let args = args.iter().collect::<Vec<_>>();

            for kv in args.chunks_exact(2) {
                let key = kv[0].to_owned();
                let value = kv[1].to_owned();

                let key = match key {
                    Value::String(key) => key,
                    _ => {
                        return Ok(Value::Error("Invalid key".to_string()));
                    }
                };

                data.insert(key, value);
            }

            store.mset(data).await;

            Value::Ok
        }

        "DEL" => {
            if args.len() < 1 {
                return Ok(Value::Error("Invalid number of arguments".to_string()));
            }

            let mut keys = vec![];

            for key in args.iter() {
                if let Value::String(key) = key {
                    keys.push(key.to_owned());
                } else {
                    return Ok(Value::Error("Invalid key".to_string()));
                }
            }

            store.remove(&keys).await
        }
        "KEYS" => {
            if args.len() > 1 {
                return Ok(Value::Error("Invalid number of arguments".to_string()));
            }

            let _pattern = match args.pop_front() {
                Some(Value::String(pattern)) => Some(pattern),
                _ => None,
            };

            store.keys().await
        }
        "EXISTS" => {
            if args.len() != 1 {
                return Ok(Value::Error("Invalid number of arguments".to_string()));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(Value::Error("Invalid key".to_string()));
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

    Ok(response)
}
