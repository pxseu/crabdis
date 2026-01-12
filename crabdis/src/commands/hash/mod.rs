pub mod hdel;
pub mod hexists;
pub mod hget;
pub mod hgetall;
pub mod hlen;
pub mod hset;

use super::{CommandMap, register_command};

pub fn register(cmds: &mut CommandMap) {
    register_command(cmds, hset::HSet);
    register_command(cmds, hgetall::HGetAll);
    register_command(cmds, hget::HGet);
    register_command(cmds, hdel::HDel);
    register_command(cmds, hexists::HExists);
    register_command(cmds, hlen::HLen);
}
