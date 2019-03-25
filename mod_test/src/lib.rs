extern crate shared;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd(
        "test",
        Command::new(|ctx: &mut Context, _args| {
            std::thread::sleep(std::time::Duration::from_secs(10));
            ctx.reply(&format!("beep boop {}", ctx.perms()?))
        }),
    );
    meta
}
