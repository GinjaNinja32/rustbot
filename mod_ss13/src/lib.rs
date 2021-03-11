use rustbot::prelude::*;

mod status;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("address", Command::new(status::address));
    meta.cmd("admins", Command::new(status::admins));
    meta.cmd("manifest", Command::new(status::manifest));
    meta.cmd("mode", Command::new(status::mode));
    meta.cmd("players", Command::new(status::players));
    meta.cmd("revision", Command::new(status::revision));
    meta.cmd("status", Command::new(status::status));
}
