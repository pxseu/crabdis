use std::time::SystemTime;

use tokio::time::{Duration, Instant};

use crate::prelude::*;

pub struct Set;

#[derive(Default, Debug)]
struct Arguments {
    pub set_nx: bool,
    pub set_xx: bool,
    pub get: bool,
    pub ex: Option<i64>,
    pub px: Option<i64>,
    pub exat: Option<i64>,
    pub pxat: Option<i64>,
    pub keepttl: bool,
}

#[async_trait]
impl CommandTrait for Set {
    fn name(&self) -> &str {
        "SET"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() < 2 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let key = match args.pop_front() {
            Some(Value::String(key)) => key,
            Some(_) => {
                return value_error!("Invalid key").to_resp2(writer).await;
            }
            None => {
                return value_error!("Missing key").to_resp2(writer).await;
            }
        };

        let value = match args.pop_front() {
            Some(value) => value,
            _ => {
                return value_error!("Missing value").to_resp2(writer).await;
            }
        };

        let mut arguments = Arguments::default();
        let mut prev_arg = None;

        for arg in args.iter() {
            match arg {
                Value::String(arg) => match arg.to_uppercase().as_str() {
                    "NX" if !arguments.set_xx => {
                        arguments.set_nx = true;
                    }
                    "XX" if !arguments.set_nx => {
                        arguments.set_xx = true;
                    }
                    "GET" => arguments.get = true,
                    "KEEPTTL"
                        if !arguments.ex.is_some()
                            && !arguments.px.is_some()
                            && !arguments.exat.is_some()
                            && !arguments.pxat.is_some() =>
                    {
                        arguments.keepttl = true;
                    }

                    "EX" | "PX" | "EXAT" | "PXAT" if !arguments.keepttl => {
                        prev_arg = Some(arg);
                    }
                    arg => {
                        if let Some(prev) = prev_arg {
                            match prev.as_str() {
                                "EX" => arguments.ex = arg.parse::<i64>().ok(),
                                "PX" => arguments.px = arg.parse::<i64>().ok(),
                                "EXAT" => arguments.exat = arg.parse::<i64>().ok(),
                                "PXAT" => arguments.pxat = arg.parse::<i64>().ok(),
                                _ => {}
                            }

                            prev_arg = None;
                            continue;
                        }

                        return value_error!("Invalid argument {arg}")
                            .to_resp2(writer)
                            .await;
                    }
                },
                _ => {
                    return value_error!("Invalid argument").to_resp2(writer).await;
                }
            }
        }

        let mut lock = session.state.store.write().await;

        let prev_key = lock.entry(key.clone()).or_insert(Value::Nil.clone());

        if arguments.set_nx && prev_key.is_some() {
            return Value::Nil.to_resp2(writer).await;
        }

        if arguments.set_xx && prev_key.is_none() {
            return Value::Nil.to_resp2(writer).await;
        }

        let expire_at = if arguments.keepttl {
            match prev_key {
                Value::Expire((_, expire_at)) => Some(*expire_at),
                _ => None,
            }
        } else {
            match (arguments.ex, arguments.px, arguments.exat, arguments.pxat) {
                (Some(ex), _, _, _) => Some(Instant::now() + Duration::from_secs(ex as u64)),
                (_, Some(px), _, _) => Some(Instant::now() + Duration::from_millis(px as u64)),
                (_, _, Some(exat), _) => Some(
                    Instant::now() + Duration::from_secs(exat as u64)
                        - SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap(),
                ),
                (_, _, _, Some(pxat)) => Some(
                    Instant::now() + Duration::from_millis(pxat as u64)
                        - SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap(),
                ),
                _ => None,
            }
        };

        if arguments.get {
            prev_key.to_resp2(writer).await?
        }

        *prev_key = if let Some(expire_at) = expire_at {
            session.state.expire_keys.write().await.insert(key);
            Value::Expire((Box::new(value), expire_at))
        } else {
            value
        };

        if arguments.get {
            return Ok(());
        }

        Value::Ok.to_resp2(writer).await
    }
}
