mod expire;
mod persist;
mod psettex;
mod pttl;
mod setex;
mod ttl;

use super::{CommandMap, register_command};

pub fn register(cmds: &mut CommandMap) {
    register_command(cmds, expire::Expire);
    register_command(cmds, ttl::Ttl);
    register_command(cmds, setex::SetEx);
    register_command(cmds, psettex::PSetEx);
    register_command(cmds, pttl::PTtl);
    register_command(cmds, persist::Persist);
}
