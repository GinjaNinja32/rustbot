#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rustbot;

use regex::Regex;
use rustbot::prelude::*;
use std::borrow::Cow;

mod format;
#[cfg(tests)]
mod tests;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("bridge", Command::new(bridge).req_perms(Perms::Admin));

    meta.handle(HandleType::All, Box::new(do_bridge));
}

lazy_static! {
    static ref ANTIPING_RE: Regex = Regex::new(r"\b[a-zA-Z0-9]").unwrap();
}

fn bridge(ctx: &dyn Context, args: &str) -> Result<()> {
    let mut db = ctx.bot().sql().lock();
    if args.is_empty() {
        let key = db.query(
            "SELECT bridge_key FROM mod_bridge WHERE config_id = $1 AND channel_id = $2",
            &[&ctx.config_id(), &ctx.source().channel_string()],
        )?;
        if key.is_empty() {
            return ctx.say("no bridge key found");
        }

        let conf = ctx.config_id();
        let chan = ctx.source().channel_string();

        let chans = db.query(
            "SELECT config_id, channel_id FROM mod_bridge WHERE bridge_key = (SELECT bridge_key FROM mod_bridge WHERE config_id = $1 AND channel_id = $2) AND config_id != $1 AND channel_id != $2",
            &[&conf, &chan],
        )?;
        let chans_str = chans
            .iter()
            .map(|row| format!("{}:{}", row.get::<_, String>(0), row.get::<_, String>(1)))
            .collect::<Vec<_>>();

        ctx.say(&format!(
            "bridge key '{}', bridged channels: {:?}",
            key.get(0).unwrap().get::<_, String>(0),
            chans_str
        ))
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

fn do_bridge(ctx: &dyn Context, _typ: HandleType, msg: &str) -> Result<()> {
    let conf = ctx.config_id();
    let chan = ctx.source().channel_string();

    let chans = {
        let mut db = ctx.bot().sql().lock();

        db.query(
            "SELECT config_id, channel_id FROM mod_bridge WHERE bridge_key = (SELECT bridge_key FROM mod_bridge WHERE config_id = $1 AND channel_id = $2) AND (config_id != $1 OR channel_id != $2)",
            &[&conf, &chan]
        )?
    };
    if chans.is_empty() {
        return Ok(());
    }

    let (user, spans): (&dyn Fn(Cow<'_, str>) -> Span<'_>, Vec<Span>) =
        if let Some((Some(g), _, _)) = ctx.source().get_discord_params() {
            (
                &|user| span!(Format::Bold; "<{}>", user),
                spans! {ctx.bot().dis_unprocess_message(conf, &format!("{}", g), msg)?},
            )
        } else if ctx.source().get_irc_params().is_some() {
            if msg.starts_with(1 as char) && msg.ends_with(1 as char) {
                let ctcp = &msg[1..msg.len() - 1];
                let parts = ctcp.splitn(2, ' ').collect::<Vec<_>>();
                match parts[0] {
                    "ACTION" => {
                        let v = format::irc_parse(parts[1]);
                        (&|user| span!(Format::Bold; "* {}", user), v)
                    }
                    _ => {
                        warn!("unexpected CTCP message {:?} {:?} in do_bridge", parts[0], parts[1]);
                        return Ok(());
                    }
                }
            } else {
                let v = format::irc_parse(msg);
                (&|user| span!(Format::Bold; "<{}>", user), v)
            }
        } else {
            (&|user| span!(Format::Bold; "<{}>", user), spans! {msg})
        };

    for row in chans.iter() {
        let tconf = row.get::<_, String>(0);
        let tchan = row.get::<_, String>(1);

        if tchan.starts_with("irc:") {
            let user_pretty = ctx.source().user_pretty();

            let user_pretty = ANTIPING_RE.replace_all(&user_pretty, "$0\u{feff}");

            let msg = Message::Prefixed(spans! {user(user_pretty), " "}, spans.clone());
            ctx.bot().send_message(&tconf, &tchan, msg)?;
        } else {
            let msg = Message::Prefixed(spans! {user(ctx.source().user_pretty()), " "}, spans.clone());
            ctx.bot().send_message(&tconf, &tchan, msg)?;
        }
    }
    Ok(())
}
