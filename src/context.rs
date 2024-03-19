use std::sync::Arc;

use crate::commands::CommandHandler;
use crate::prelude::*;

#[derive(Clone, Default)]
pub struct Context {
    pub store: Store,
    pub commands: CommandHandler,
}

impl Context {
    pub async fn new() -> Arc<Self> {
        let mut context = Context::default();

        context.commands.register().await;

        Arc::new(context)
    }
}

pub type ContextRef = Arc<Context>;
