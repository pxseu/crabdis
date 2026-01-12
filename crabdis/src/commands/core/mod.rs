mod client;
mod command;
mod dbsize;
mod decr;
mod del;
mod exists;
mod flushdb;
mod get;
mod hello;
mod incr;
mod info;
mod keys;
mod mget;
mod mset;
mod ping;
mod quit;
mod renamenx;
mod scan;
mod select;
mod set;
mod r#type;

use super::{CommandMap, register_command};

pub fn register(cmds: &mut CommandMap) {
    register_command(cmds, client::Client);
    register_command(cmds, command::Command);
    register_command(cmds, dbsize::DBSize);
    register_command(cmds, get::Get);
    register_command(cmds, set::Set);
    register_command(cmds, del::Del);
    register_command(cmds, mget::MGet);
    register_command(cmds, ping::Ping);
    register_command(cmds, mset::MSet);
    register_command(cmds, keys::Keys);
    register_command(cmds, hello::Hello);
    register_command(cmds, exists::Exists);
    register_command(cmds, flushdb::FlushDB);
    register_command(cmds, info::Info);
    register_command(cmds, incr::Incr);
    register_command(cmds, decr::Decr);
    register_command(cmds, scan::Scan);
    register_command(cmds, r#type::Type);
    register_command(cmds, select::Select);
    register_command(cmds, renamenx::RenameNx);
    register_command(cmds, quit::Quit);
}
