use crate::commands::COMMANDS;
use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 4,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Set the value and expiration of a key in seconds",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct SetEx;

#[async_trait]
impl Handler for SetEx {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
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

        let set_args = [key, value, Value::String("EX".into()), seconds];
        let mut set_args = Args::new(&set_args);

        let set_cmd = COMMANDS.get("SET").unwrap();
        set_cmd.handle(writer, &mut set_args, session).await
    }
}
