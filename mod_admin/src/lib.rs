extern crate shared;

use shared::types;
use shared::types::Source::*;
use std::rc::Rc;

#[no_mangle]
pub fn get_meta() -> types::Meta {
	let mut meta = types::Meta::new();
	meta.commandrc("raw", Rc::new(wrap(raw)));
	meta.command("whoami", whoami);
	meta
}

fn wrap(f: impl Fn(&mut types::Context, &str)) -> impl Fn(&mut types::Context, &str) {
    move |ctx: &mut types::Context, args| {
        if ctx.has_perm("admin") {
            f(ctx, args)
        } else {
            ctx.reply("permission denied")
        }
    }
}

fn raw(ctx: &mut types::Context, args: &str) {
    ctx.bot().send_raw(args);
}

fn whoami(ctx: &mut types::Context, _: &str) {
    match ctx.get_source() {
        None => ctx.reply(&format!("I don't know who you are")),
        Some(Server(s)) => ctx.reply(&format!("You are {}", s)),
        Some(User{nick, user, host}) => ctx.reply(&format!("You are {}!{}@{}", nick, user, host)),
    }
    ctx.reply(&format!("Admin: {}", ctx.has_perm("admin")));
}
