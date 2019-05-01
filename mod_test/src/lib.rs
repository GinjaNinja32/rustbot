extern crate shared;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd(
        "test",
        Command::new(|ctx, args| {
            ctx.say(&format!("beep boop {}", ctx.perms()?))?;
            ctx.say(&format!("you passed: {}", args))
        }),
    );
    meta
}
