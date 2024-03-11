use crate::prelude::*;

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
                return Ok(value_error!("Invalid number of arguments"));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(value_error!("Invalid key"));
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
                    return Ok(value_error!("Invalid key"));
                }
            }

            store.mget(&keys).await
        }

        "SET" => {
            if args.len() != 2 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(value_error!("Invalid key"));
                }
            };

            let value = match args.pop_front() {
                Some(value) => value,
                _ => {
                    return Ok(value_error!("Invalid value"));
                }
            };

            store.set(key, value).await;

            Value::Ok
        }

        "MSET" => {
            let mut data = HashMap::new();

            if args.len() % 2 != 0 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            for kv in args.iter().collect::<Vec<_>>().chunks_exact(2) {
                let key = match kv[0].to_owned() {
                    Value::String(key) => key,
                    _ => {
                        return Ok(value_error!("Invalid key"));
                    }
                };

                data.insert(key, kv[1].to_owned());
            }

            store.mset(data).await;

            Value::Ok
        }

        "DEL" => {
            if args.len() < 1 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let mut keys = vec![];

            for key in args.iter() {
                if let Value::String(key) = key {
                    keys.push(key.to_owned());
                } else {
                    return Ok(value_error!("Invalid key"));
                }
            }

            store.del(&keys).await
        }

        "KEYS" => {
            if args.len() > 1 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let _pattern = match args.pop_front() {
                Some(Value::String(pattern)) => Some(pattern),
                _ => None,
            };

            store.keys().await
        }

        "EXISTS" => {
            if args.len() != 1 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(value_error!("Invalid key"));
                }
            };

            let exists = store.exists(&key).await;

            Value::Integer(exists as i64)
        }

        "HGET" => {
            if args.len() != 2 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(value_error!("Invalid key"));
                }
            };

            let field = match args.pop_front() {
                Some(Value::String(field)) => field,
                _ => {
                    return Ok(value_error!("Invalid field"));
                }
            };

            store.hget(&key, &field).await
        }

        "HSET" => {
            if args.len() < 3 || args.len() % 2 != 1 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(value_error!("Invalid key"));
                }
            };

            let mut hashmap = HashMap::new();

            for kv in args.iter().collect::<Vec<_>>().chunks_exact(2) {
                let field = match kv[0].to_owned() {
                    Value::String(field) => field,
                    _ => {
                        return Ok(value_error!("Invalid field"));
                    }
                };

                hashmap.insert(field, kv[1].to_owned());
            }

            store.hset(key, hashmap).await
        }

        "HGETALL" => {
            if args.len() != 1 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            let key = match args.pop_front() {
                Some(Value::String(key)) => key,
                _ => {
                    return Ok(value_error!("Invalid key"));
                }
            };

            store.hgetall(&key).await
        }

        "FLUSHDB" => {
            // there is a weird asnyc/sync arg which is pointless here
            if args.len() > 1 {
                return Ok(value_error!("Invalid number of arguments"));
            }

            store.clear().await;

            Value::Ok
        }

        _ => value_error!("Unknown command"),
    };

    Ok(response)
}
