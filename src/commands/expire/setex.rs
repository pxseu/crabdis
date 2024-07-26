use crate::prelude::*;

pub struct SetEx;

#[async_trait]
impl CommandTrait for SetEx {
    fn name(&self) -> &str {
        "SETEX"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 3 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        // add EX to the arguments and call SET command
        args.insert(2, Value::String("EX".to_string()));

        let clone = session.clone();

        let commands = clone.state.handler.commands.read().await;

        let set_cmd = commands.get("SET").unwrap();

        set_cmd.handle_command(writer, args, session).await
    }
}
