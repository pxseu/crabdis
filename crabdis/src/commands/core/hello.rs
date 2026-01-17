use crate::prelude::*;

pub struct Hello;

#[async_trait]
impl CommandTrait for Hello {
    fn name(&self) -> &'static str {
        "HELLO"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Handshake with the Redis server",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() > 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        if let Some(version) = args.next_string() {
            if version.as_ref() != "2" && version.as_ref() != "3" {
                return session
                    .respond(&value_error!("Invalid version"), writer)
                    .await;
            }

            // above check ensures that the version is either 2 or 3
            let version = version.as_bytes()[0] - b'0';

            session.set_proto(version);
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
                Value::Integer(session.proto().into()),
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
