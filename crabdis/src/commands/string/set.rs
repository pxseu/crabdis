use std::time::SystemTime;

use tokio::time::{Duration, Instant};

use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -3,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Set the string value of a key",
    complexity = "O(1)",
    since = "0.1.34",
)]
pub struct Set;

#[allow(clippy::struct_excessive_bools)]
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
impl Handler for Set {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() < 2 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let Some(value) = args.next_owned() else {
            return session
                .respond(&value_error!("Missing value"), writer)
                .await;
        };

        let mut arguments = Arguments::default();
        let mut prev_ex_arg = None;

        for arg in args {
            match arg {
                Value::String(arg) => match arg.to_uppercase().as_str() {
                    "NX" if !arguments.set_xx => {
                        arguments.set_nx = true;
                    }
                    "XX" if !arguments.set_nx => {
                        arguments.set_xx = true;
                    }
                    "GET" => arguments.get = true,
                    "KEEPTTL" if prev_ex_arg.is_none() => {
                        arguments.keepttl = true;
                    }

                    // also check if the previous argument was EX, PX, EXAT, PXAT
                    "EX" | "PX" | "EXAT" | "PXAT"
                        if !arguments.keepttl && prev_ex_arg.is_none() =>
                    {
                        prev_ex_arg = Some(arg);
                    }
                    arg => {
                        if let Some(prev) = prev_ex_arg {
                            match prev.as_ref() {
                                "EX" => arguments.ex = arg.parse::<i64>().ok(),
                                "PX" => arguments.px = arg.parse::<i64>().ok(),
                                "EXAT" => arguments.exat = arg.parse::<i64>().ok(),
                                "PXAT" => arguments.pxat = arg.parse::<i64>().ok(),
                                _ => {}
                            }

                            continue;
                        }

                        return session
                            .respond(&value_error!("Invalid argument {arg}"), writer)
                            .await;
                    }
                },
                _ => {
                    return session
                        .respond(&value_error!("Invalid argument"), writer)
                        .await;
                }
            }
        }

        let mut lock = session.state.store.write().await;

        let prev_key = lock.entry(key.clone()).or_insert(Value::Nil.clone());

        if arguments.set_nx && prev_key.is_some() {
            return session.respond(&Value::Nil, writer).await;
        }

        if arguments.set_xx && prev_key.is_none() {
            return session.respond(&Value::Nil, writer).await;
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
                // SAFETY: `SystemTime::now()` is always after `SystemTime::UNIX_EPOCH`. If it's not
                // YOU HAVE A BIGGER PROBLEM
                (_, _, Some(exat), _) => Some(
                    Instant::now() + Duration::from_secs(exat as u64)
                        - SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .expect("SystemTime before UNIX_EPOCH"),
                ),
                (_, _, _, Some(pxat)) => Some(
                    Instant::now() + Duration::from_millis(pxat as u64)
                        - SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .expect("SystemTime before UNIX_EPOCH"),
                ),
                _ => None,
            }
        };

        if arguments.get {
            session.respond(prev_key, writer).await?;
        } else {
            session.respond(&Value::Ok, writer).await?;
        }

        *prev_key = value;

        if let Some(expires_at) = expire_at {
            session.state.expire_keys.write().await.insert(key);
            prev_key.set_expire(expires_at);
        }

        // Notify state that data changed (for RDB auto-save)
        session.state.notify_change();

        Ok(())
    }
}
