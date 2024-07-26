use crate::prelude::*;

pub struct PSetEx;

#[async_trait]
impl CommandTrait for PSetEx {
    fn name(&self) -> &str {
        "PSETEX"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        // add EX to the arguments and call SET command
        args.insert(2, Value::String("PX".to_string()));

        let clone = session.clone();

        let commands = clone.state.handler.commands.read().await;

        let set_cmd = commands.get("SET").unwrap();

        set_cmd.handle_command(writer, args, session).await
    }
}
