extern crate postgres;
extern crate rustbot;
extern crate serde_json;

use rustbot::prelude::*;

mod bash;
mod db;
mod raw;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();

    meta.cmd("raw", Command::new(raw::raw).req_perms(Perms::Raw));
    meta.cmd("join", Command::new(raw::join).req_perms(Perms::Raw));
    meta.cmd("part", Command::new(raw::part).req_perms(Perms::Raw));
    meta.cmd("dmsg", Command::new(raw::dmsg).req_perms(Perms::Raw));
    meta.cmd("imsg", Command::new(raw::imsg).req_perms(Perms::Raw));

    meta.cmd("q", Command::new(db::query).req_perms(Perms::Database));

    meta.cmd("whoami", Command::new(whoami));

    meta.cmd("bash", Command::new(bash::bash).req_perms(Perms::Eval));
    meta.cmd("bashl", Command::new(bash::bashl).req_perms(Perms::Eval));

    meta
}

fn whoami(ctx: &Context, _: &str) -> Result<()> {
    match ctx.source {
        IRC { ref prefix, .. } => {
            if let Some(p) = prefix {
                ctx.reply(Message::Simple(format!(
                    "You are {}:{}\nFlags: {}",
                    ctx.config,
                    p,
                    ctx.perms()?
                )))?;
            }
        }
        Discord { guild, ref user, .. } => ctx.reply(Message::Simple(format!(
            "You are {}:{:?}:{}\nFlags: {}",
            ctx.config,
            guild.map(|g| *g.as_u64()),
            user.id.as_u64(),
            ctx.perms()?
        )))?,
    }
    Ok(())
}
