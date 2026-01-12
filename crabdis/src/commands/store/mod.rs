mod bgsave;
mod debug;
mod lastsave;
mod save;

use super::{CommandMap, register_command};

pub fn register(cmds: &mut CommandMap) {
    register_command(cmds, save::Save);
    register_command(cmds, bgsave::BgSave);
    register_command(cmds, lastsave::LastSave);
    register_command(cmds, debug::Debug);
}
