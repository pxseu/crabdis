use crate::prelude::*;

pub struct Auth;

#[async_trait]
impl CommandTrait for Auth {
    fn name(&self) -> &'static str {
        "AUTH"
    }

    fn requires_auth(&self) -> bool {
        false
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Authenticate to the server",
            complexity: "O(1)",
            since: "0.1.36",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if !(1..=2).contains(&args.len()) {
            return session
                .respond(&value_error!("ERR Invalid number of arguments"), writer)
                .await;
        }

        let (username, password) = match (args.next_string(), args.next_string()) {
            (Some(p), None) => (&Arc::from("default"), p),
            (Some(u), Some(p)) => (u, p),
            _ => {
                return session
                    .respond(&value_error!("ERR Invalid number of arguments"), writer)
                    .await;
            }
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

        session.respond(&Value::Ok, writer).await
    }
}
