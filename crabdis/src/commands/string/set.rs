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

#[derive(Clone, Copy)]
enum ExpireArg {
    Ex,
    Px,
    ExAt,
    PxAt,
}

fn parse_arguments(args: &mut Args<'_>) -> std::result::Result<Arguments, Value> {
    let mut arguments = Arguments::default();
    let mut prev_ex_arg = None;

    for arg in args {
        match arg {
            Value::String(arg) => {
                if arg.eq_ignore_ascii_case("NX") && !arguments.set_xx {
                    arguments.set_nx = true;
                } else if arg.eq_ignore_ascii_case("XX") && !arguments.set_nx {
                    arguments.set_xx = true;
                } else if arg.eq_ignore_ascii_case("GET") {
                    arguments.get = true;
                } else if arg.eq_ignore_ascii_case("KEEPTTL") && prev_ex_arg.is_none() {
                    arguments.keepttl = true;
                } else if !arguments.keepttl && prev_ex_arg.is_none() {
                    prev_ex_arg = if arg.eq_ignore_ascii_case("EX") {
                        Some(ExpireArg::Ex)
                    } else if arg.eq_ignore_ascii_case("PX") {
                        Some(ExpireArg::Px)
                    } else if arg.eq_ignore_ascii_case("EXAT") {
                        Some(ExpireArg::ExAt)
                    } else if arg.eq_ignore_ascii_case("PXAT") {
                        Some(ExpireArg::PxAt)
                    } else {
                        None
                    };

                    if prev_ex_arg.is_none() {
                        return Err(value_error!("Invalid argument {arg}"));
                    }
                } else {
                    if let Some(prev) = prev_ex_arg {
                        match prev {
                            ExpireArg::Ex => arguments.ex = arg.parse::<i64>().ok(),
                            ExpireArg::Px => arguments.px = arg.parse::<i64>().ok(),
                            ExpireArg::ExAt => arguments.exat = arg.parse::<i64>().ok(),
                            ExpireArg::PxAt => arguments.pxat = arg.parse::<i64>().ok(),
                        }

                        prev_ex_arg = None;
                        continue;
                    }

                    return Err(value_error!("Invalid argument {arg}"));
                }
            }
            _ => return Err(value_error!("Invalid argument")),
        }
    }

    Ok(arguments)
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

        let arguments = match parse_arguments(args) {
            Ok(arguments) => arguments,
            Err(error) => return session.respond(&error, writer).await,
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_expiration_followed_by_condition() {
        let values = [
            Value::String("ex".into()),
            Value::String("10".into()),
            Value::String("nx".into()),
            Value::String("get".into()),
        ];
        let mut args = Args::new(&values);

        let parsed = parse_arguments(&mut args).unwrap();

        assert_eq!(parsed.ex, Some(10));
        assert!(parsed.set_nx);
        assert!(parsed.get);
    }
}
