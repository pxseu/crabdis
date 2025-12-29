use crate::prelude::*;

pub struct SetEx;

#[async_trait]
impl CommandTrait for SetEx {
    fn name(&self) -> &str {
        "SETEX"
    }

    async fn handle_command(
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

        // SETEX key seconds value -> SET key value EX seconds
        let key = args.next_owned().unwrap();
        let seconds = args.next_owned().unwrap();
        let value = args.next_owned().unwrap();

        let set_args = vec![key, value, Value::String("EX".into()), seconds];
        let mut set_args = Args::new(&set_args);

        let commands = session.state.handler.commands.read().await;
        let set_cmd = commands.get("SET").unwrap();

        set_cmd
            .handle_command(writer, &mut set_args, session.clone())
            .await
    }
}
