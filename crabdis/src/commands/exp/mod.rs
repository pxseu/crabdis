mod expire;
mod persist;
mod psettex;
mod pttl;
mod setex;
mod ttl;

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(expire::Expire);
    cmds.register(ttl::Ttl);
    cmds.register(setex::SetEx);
    cmds.register(psettex::PSetEx);
    cmds.register(pttl::PTtl);
    cmds.register(persist::Persist);
}
