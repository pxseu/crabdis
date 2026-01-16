mod hdel;
mod hexists;
mod hget;
mod hgetall;
mod hlen;
mod hset;

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(hset::HSet);
    cmds.register(hgetall::HGetAll);
    cmds.register(hget::HGet);
    cmds.register(hdel::HDel);
    cmds.register(hexists::HExists);
    cmds.register(hlen::HLen);
}
