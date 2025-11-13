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
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        if let Some(Value::String(version)) = args.pop_front() {
            if version.as_ref() != "2" && version.as_ref() != "3" {
                return session
                    .versioned_response(&value_error!("Invalid version"), writer)
                    .await;
            }

            // safe to unwrap since we've already checked the value
            session
                .set_proto_version(version.parse::<u8>().unwrap())
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
                Value::Integer(session.get_proto_version().await.into()),
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

        session.versioned_response(&response, writer).await
    }
}
