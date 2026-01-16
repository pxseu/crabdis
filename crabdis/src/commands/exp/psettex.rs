use crate::commands::COMMANDS;
use crate::prelude::*;

pub struct PSetEx;

#[async_trait]
impl CommandTrait for PSetEx {
    fn name(&self) -> &'static str {
        "PSETEX"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 4,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Set the value and expiration in milliseconds of a key",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 3 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        // PSETEX key milliseconds value -> SET key value PX milliseconds
        let key = args.next_owned().unwrap();
        let milliseconds = args.next_owned().unwrap();
        let value = args.next_owned().unwrap();

        let set_args = vec![key, value, Value::String("PX".into()), milliseconds];
        let mut set_args = Args::new(&set_args);

        let set_cmd = COMMANDS.get(&"SET".into()).unwrap();
        set_cmd.handle(writer, &mut set_args, session.clone()).await
    }
}
