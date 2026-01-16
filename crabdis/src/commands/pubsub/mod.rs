mod publish;
mod subscribe;
mod unsubscribe;

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(publish::Publish);
    cmds.register(subscribe::Subscribe);
    cmds.register(unsubscribe::Unsubscribe);
}
