extern crate postgres;
extern crate rustbot;
extern crate serde_json;

use rustbot::prelude::*;

mod bash;
mod db;
mod raw;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("raw", Command::new(raw::raw).req_perms(Perms::Raw));
    meta.cmd("join", Command::new(raw::join).req_perms(Perms::Raw));
    meta.cmd("part", Command::new(raw::part).req_perms(Perms::Raw));
    meta.cmd("dmsg", Command::new(raw::dmsg).req_perms(Perms::Raw));
    meta.cmd("imsg", Command::new(raw::imsg).req_perms(Perms::Raw));

    meta.cmd("q", Command::new(db::query).req_perms(Perms::Database));

    meta.cmd("whoami", Command::new(whoami));

    meta.cmd("bash", Command::new(bash::bash).req_perms(Perms::Eval));
    meta.cmd("bashl", Command::new(bash::bashl).req_perms(Perms::Eval));
}

fn whoami(ctx: &dyn Context, _: &str) -> Result<()> {
    ctx.reply(Message::Simple(format!(
        "You are {}\nFlags: {}",
        ctx.source_str(),
        ctx.perms()?
    )))
}
