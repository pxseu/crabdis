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

use crate::prelude::*;

pub fn register(cmds: &mut CommandRegistry) {
    cmds.register(get::Get);
    cmds.register(renamenx::RenameNx);
    cmds.register(scan::Scan);
    cmds.register(select::Select);
    cmds.register(client::Client);
    cmds.register(command::Command);
    cmds.register(dbsize::DBSize);
    cmds.register(decr::Decr);
    cmds.register(del::Del);
    cmds.register(exists::Exists);
    cmds.register(flushdb::FlushDB);
    cmds.register(hello::Hello);
    cmds.register(incr::Incr);
    cmds.register(info::Info);
    cmds.register(keys::Keys);
    cmds.register(mget::MGet);
    cmds.register(mset::MSet);
    cmds.register(ping::Ping);
    cmds.register(quit::Quit);
    cmds.register(set::Set);
    cmds.register(r#type::Type);
}
