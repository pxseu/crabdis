use crate::prelude::*;

#[derive(Command)]
#[command(
    noauth,
    arity = -1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Handshake with the Redis server",
    complexity = "O(1)",
    since = "0.1.34",
)]
pub struct Hello;

#[async_trait]
impl Handler for Hello {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        // Parse optional protover
        if let Some(raw_version) = args.next() {
            let Ok(version) = Version::from_value(raw_version) else {
                return session
                    .respond(
                        &value_error!("NOPROTO unsupported protocol version"),
                        writer,
                    )
                    .await;
            };

            session.set_proto(version);

            // Parse optional arguments: AUTH username password, SETNAME clientname
            while let Some(option) = args.next_string() {
                if option.eq_ignore_ascii_case("AUTH") {
                    let first = args.next_string();
                    let second = args.next_string();

                    let (Some(username), Some(password)) = (first, second) else {
                        return session
                            .respond(
                                &value_error!("ERR wrong number of arguments for 'AUTH' in HELLO"),
                                writer,
                            )
                            .await;
                    };

                    if session.state.auth.login(username, password).is_err() {
                        return session
                            .respond(
                                &value_error!("WRONGPASS invalid username-password pair"),
                                writer,
                            )
                            .await;
                    }

                    session.set_authenticated(true);
                } else if option.eq_ignore_ascii_case("SETNAME") {
                    let Some(name) = args.next_string_owned() else {
                        return session
                            .respond(
                                &value_error!("ERR wrong number of arguments for 'SETNAME' in HELLO"),
                                writer,
                            )
                            .await;
                    };

                    session.set_name(name).await;
                } else {
                    return session
                        .respond(&value_error!("ERR unknown option '{option}'"), writer)
                        .await;
                }
            }
        }

        // Check if authentication is required but not provided
        if session.state.auth.has_auth() && !session.is_authenticated() {
            return session
                .respond(&value_error!("NOAUTH HELLO must be called with the client already authenticated, otherwise the HELLO <proto> AUTH <user> <pass> option can be used to authenticate the client and select the RESP protocol version at the same time"), writer)
                .await;
        }

        let response = Value::Map(HashMap::from([
            (
                Value::String("server".into()),
                Value::String(env!("CARGO_PKG_NAME").into()),
            ),
            (
                Value::String("version".into()),
                Value::String(env!("CARGO_PKG_VERSION").into()),
            ),
            (
                Value::String("proto".into()),
                Value::Integer(i64::from(session.proto().as_u8())),
            ),
            (
                Value::String("id".into()),
                Value::Integer(session.id as i64),
            ),
            (
                Value::String("mode".into()),
                Value::String("standalone".into()),
            ),
            (Value::String("role".into()), Value::String("master".into())),
            (
                Value::String("modules".into()),
                Value::Multi(Arc::default()),
            ),
        ]));

        session.respond(&response, writer).await
    }
}
