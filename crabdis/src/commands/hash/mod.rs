mod hdel;
mod hexists;
mod hget;
mod hgetall;
mod hlen;
mod hset;

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(hdel::HDel);
    cmds.register(hexists::HExists);
    cmds.register(hget::HGet);
    cmds.register(hgetall::HGetAll);
    cmds.register(hlen::HLen);
    cmds.register(hset::HSet);
}
