use crate::prelude::*;

pub struct Info;

#[async_trait]
impl CommandTrait for Info {
    fn name(&self) -> &str {
        "INFO"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        _session: SessionRef,
    ) -> Result<()> {
        if args.len() > 0 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let response = Value::String("loading:0".to_string());

        response.to_resp2(writer).await
    }
}
