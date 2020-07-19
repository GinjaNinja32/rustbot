#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate rustbot;

use rustbot::prelude::*;

mod format;
#[cfg(tests)]
mod tests;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("bridge", Command::new(bridge).req_perms(Perms::Admin));

    meta.handle(HandleType::All, Box::new(do_bridge));
}

fn bridge(ctx: &dyn Context, args: &str) -> Result<()> {
    let db = ctx.bot().sql().lock();
    if args == "" {
        let key = db.query(
            "SELECT bridge_key FROM mod_bridge WHERE config_id = $1 AND channel_id = $2",
            &[&ctx.config_id(), &ctx.source().channel_string()],
        )?;
        if key.is_empty() {
            return ctx.say("no bridge key found");
        }

        ctx.say(&format!("bridge key: '{}'", key.get(0).get::<_, String>(0)))
    } else if args == "none" {
        let n = db.execute(
            "DELETE FROM mod_bridge WHERE config_id = $1 AND channel_id = $2",
            &[&ctx.config_id(), &ctx.source().channel_string()],
        )?;
        if n != 1 {
            ctx.say("there is no bridge key to clear")
        } else {
            ctx.say("bridge key cleared")
        }
    } else {
        db.execute(
            "INSERT INTO mod_bridge (config_id, channel_id, bridge_key) VALUES ($1, $2, $3) ON CONFLICT (config_id, channel_id) DO UPDATE SET bridge_key = $3",
            &[&ctx.config_id(), &ctx.source().channel_string(), &args],
        )?;

        ctx.say(&format!("bridge key set to '{}'", args))
    }
}

fn do_bridge(ctx: &dyn Context, msg: &str) -> Result<()> {
    let db = ctx.bot().sql().lock();

    let conf = ctx.config_id();
    let chan = ctx.source().channel_string();

    let chans = db.query(
        "SELECT config_id, channel_id FROM mod_bridge WHERE bridge_key = (SELECT bridge_key FROM mod_bridge WHERE config_id = $1 AND channel_id = $2) AND config_id != $1 AND channel_id != $2",
        &[&conf, &chan],
    )?;
    if chans.is_empty() {
        return Ok(());
    }

    let user = span!(Format::Bold; "<{}>", ctx.source().user_pretty());

    let msg = Message::Spans(if let Some((Some(g), _, _)) = ctx.source().get_discord_params() {
        spans! {user, " ", ctx.bot().dis_unprocess_message(conf, &format!("{}", g), &msg)?}
    } else if ctx.source().get_irc_params().is_some() {
        let v = format::irc_parse(msg);
        spans! {user, " ", v}
    } else {
        spans! {user, " ", msg}
    });

    for row in chans.iter() {
        let tconf = row.get::<_, String>(0);
        let tchan = row.get::<_, String>(1);

        ctx.bot().send_message(&tconf, &tchan, msg.clone())?;
    }
    Ok(())
}
