use crate::prelude::*;

pub struct Hello;

#[async_trait]
impl CommandTrait for Hello {
    fn name(&self) -> &str {
        "HELLO"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
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

            // safe to unwrap since we've already checked the value
            session.set_proto(version.parse::<u8>().unwrap());
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
                Value::Multi(Default::default()),
            ),
        ]));

        session.respond(&response, writer).await
    }
}
