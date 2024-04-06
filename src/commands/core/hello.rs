use crate::prelude::*;

pub struct Hello;

#[async_trait]
impl CommandTrait for Hello {
    fn name(&self) -> &str {
        "HELLO"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        match args.pop_front() {
            Some(Value::String(version)) => {
                if version != "2" && version != "3" {
                    return value_error!("Invalid version").to_resp2(writer).await;
                }

                session.set_proto_version(version.parse().unwrap()).await;
            }

            _ => {}
        }

        let response = Value::Hashmap(HashMap::from([
            (
                "server".to_string(),
                Value::String(env!("CARGO_PKG_NAME").to_string()),
            ),
            (
                "version".to_string(),
                Value::String(env!("CARGO_PKG_VERSION").to_string()),
            ),
            (
                "proto".to_string(),
                Value::Integer(session.get_proto_version().await),
            ),
            // TODO: fix no count of connected clients
            ("id".to_string(), Value::Integer(0)),
            ("mode".to_string(), Value::String("standalone".to_string())),
            ("role".to_string(), Value::String("master".to_string())),
            ("modules".to_string(), Value::Multi(Default::default())),
        ]));

        session.versioned_response(&response, writer).await
    }
}
