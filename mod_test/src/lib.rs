extern crate shared;

use shared::types;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut meta = types::Meta::new();
    meta.command("test", |ctx: &mut types::Context, _args| {
        ctx.reply(&format!("beep boop {}", ctx.perms()));
    });
    meta
}
