mod bgsave;
mod debug;
mod lastsave;
mod save;

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(save::Save);
    cmds.register(bgsave::BgSave);
    cmds.register(lastsave::LastSave);
    cmds.register(debug::Debug);
}
