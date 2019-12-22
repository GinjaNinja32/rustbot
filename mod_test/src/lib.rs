extern crate rustbot;

use rustbot::prelude::*;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd(
        "test",
        Command::new(|ctx, args| {
            ctx.say(&format!("beep boop {}", ctx.perms()?))?;
            ctx.say(&format!("you passed: {}", args))
        }),
    );
}
