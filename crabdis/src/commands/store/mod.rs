mod bgsave;
mod debug;
mod lastsave;
mod save;

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(bgsave::BgSave);
    cmds.register(debug::Debug);
    cmds.register(lastsave::LastSave);
    cmds.register(save::Save);
}
