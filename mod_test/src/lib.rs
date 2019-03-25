extern crate shared;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd(
        "test",
        Command::new(|ctx: &Context, _args| ctx.say(&format!("beep boop {}", ctx.perms()?))),
    );
    meta
}
