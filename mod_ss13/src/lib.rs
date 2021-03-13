use rustbot::prelude::*;

mod status;
mod updates;
mod utils;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("address", Command::new(status::address));
    meta.cmd("admins", Command::new(status::admins));
    meta.cmd("manifest", Command::new(status::manifest));
    meta.cmd("mode", Command::new(status::mode));
    meta.cmd("players", Command::new(status::players));
    meta.cmd("revision", Command::new(status::revision));
    meta.cmd("status", Command::new(status::status));

    meta.cmd("update?", Command::new(updates::check_update));
    meta.cmd("ss13pullrepo", Command::new(updates::pull_repo));
}

#[macro_export(crate)]
macro_rules! build_message {
    ($resp:ident, $fmt:literal, $($name:ident),*) => {
        {
            format!($fmt,
            $(
                $resp.get(stringify!($name)).unwrap_or(&::std::borrow::Cow::Borrowed("?"))
            ),*
            )
        }
    }
}
