pub mod publish;
pub mod subscribe;
pub mod unsubscribe;

use super::{CommandMap, register_command};

pub fn register(cmds: &mut CommandMap) {
    register_command(cmds, publish::Publish);
    register_command(cmds, subscribe::Subscribe);
    register_command(cmds, unsubscribe::Unsubscribe);
}
