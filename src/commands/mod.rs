pub mod core;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

#[async_trait]
pub trait CommandTrait {
    fn name(&self) -> &str;

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        context: ContextRef,
    ) -> Result<()>;
}

macro_rules! register_commands {
    ($handler:expr, $($command:expr),+ $(,)?) => {
        $(
            $handler.register_command($command).await;
        )+
    };
}

#[derive(Clone, Default)]
pub struct CommandHandler {
    commands: Arc<RwLock<HashMap<String, Box<dyn CommandTrait + Send + Sync>>>>,
}

impl CommandHandler {
    pub async fn register(&mut self) {
        register_commands!(
            self,
            core::Get,
            core::Set,
            core::Del,
            core::MGet,
            core::Ping,
            core::MSet,
            core::Keys,
            core::Exists,
            core::FlushDB,
        );
    }

    async fn register_command<C>(&mut self, command: C)
    where
        C: CommandTrait + Send + Sync + 'static,
    {
        self.commands
            .write()
            .await
            .insert(command.name().to_uppercase(), Box::new(command));
    }

    pub async fn handle_command(
        &self,
        writer: &mut WriteHalf<'_>,
        args: &mut VecDeque<Value>,
        context: ContextRef,
    ) -> Result<()> {
        let command = match args.pop_front() {
            Some(Value::String(command)) => command.to_uppercase(),
            _ => return value_error!("Invalid command").to_resp(writer).await,
        };

        match self.commands.read().await.get(&command) {
            Some(command) => command.handle_command(writer, args, context).await,
            None => value_error!("Unknown command").to_resp(writer).await,
        }
    }
}

// pub async fn handle_command(
//     command: &str,
//     args: &mut VecDeque<Value>,
//     store: &mut Store,
// ) -> Result<Value> {
//     let response = match command {

//         "MSET" => {
//             let mut data = HashMap::new();

//             if args.len() % 2 != 0 {
//                 return Ok(value_error!("Invalid number of arguments"));
//             }

//             for kv in args.iter().collect::<Vec<_>>().chunks_exact(2) {
//                 let key = match kv[0].to_owned() {
//                     Value::String(key) => key,
//                     _ => {
//                         return Ok(value_error!("Invalid key"));
//                     }
//                 };

//                 data.insert(key, kv[1].to_owned());
//             }

//             store.mset(data).await;

//             Value::Ok
//         }

//         "KEYS" => {
//             if args.len() > 1 {
//                 return Ok(value_error!("Invalid number of arguments"));
//             }

//             let _pattern = match args.pop_front() {
//                 Some(Value::String(pattern)) => Some(pattern),
//                 _ => None,
//             };

//             store.keys().await
//         }

//         "HGET" => {
//             if args.len() != 2 {
//                 return Ok(value_error!("Invalid number of arguments"));
//             }

//             let key = match args.pop_front() {
//                 Some(Value::String(key)) => key,
//                 _ => {
//                     return Ok(value_error!("Invalid key"));
//                 }
//             };

//             let field = match args.pop_front() {
//                 Some(Value::String(field)) => field,
//                 _ => {
//                     return Ok(value_error!("Invalid field"));
//                 }
//             };

//             store.hget(&key, &field).await
//         }

//         "HSET" => {
//             if args.len() < 3 || args.len() % 2 != 1 {
//                 return Ok(value_error!("Invalid number of arguments"));
//             }

//             let key = match args.pop_front() {
//                 Some(Value::String(key)) => key,
//                 _ => {
//                     return Ok(value_error!("Invalid key"));
//                 }
//             };

//             let mut hashmap = HashMap::new();

//             for kv in args.iter().collect::<Vec<_>>().chunks_exact(2) {
//                 let field = match kv[0].to_owned() {
//                     Value::String(field) => field,
//                     _ => {
//                         return Ok(value_error!("Invalid field"));
//                     }
//                 };

//                 hashmap.insert(field, kv[1].to_owned());
//             }

//             store.hset(key, hashmap).await
//         }

//         "HGETALL" => {
//             if args.len() != 1 {
//                 return Ok(value_error!("Invalid number of arguments"));
//             }

//             let key = match args.pop_front() {
//                 Some(Value::String(key)) => key,
//                 _ => {
//                     return Ok(value_error!("Invalid key"));
//                 }
//             };

//             store.hgetall(&key).await
//         }

//         _ => value_error!("Unknown command"),
//     };

//     Ok(response)
// }
