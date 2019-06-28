extern crate rusqlite;
extern crate shared;

use rusqlite::types::ValueRef::*;
use rusqlite::NO_PARAMS;
use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("raw", Command::new(raw).req_perms(Perms::Admin));
    meta.cmd("join", Command::new(join).req_perms(Perms::Admin));
    meta.cmd("part", Command::new(part).req_perms(Perms::Admin));
    meta.cmd("q", Command::new(query).req_perms(Perms::Admin));
    meta.cmd("whoami", Command::new(whoami));

    meta.cmd("dmsg", Command::new(dmsg).req_perms(Perms::Admin));
    meta.cmd("imsg", Command::new(imsg).req_perms(Perms::Admin));

    meta
}

fn dmsg(ctx: &Context, args: &str) -> Result<()> {
    let mut args: Vec<&str> = args.splitn(3, " ").collect();
    if args.len() != 3 {
        return Err(Error::new("usage: imsg <config_id> <channel> <message...>"));
    }

    if args[1].chars().collect::<Vec<char>>()[0] == '#' {
        args[1] = &args[1][1..];
    }

    ctx.bot.dis_send_message(args[0], args[1], args[2], true)
}

fn imsg(ctx: &Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, " ").collect();
    if args.len() != 3 {
        return Err(Error::new("usage: imsg <config_id> <channel> <message...>"));
    }

    ctx.bot.irc_send_privmsg(args[0], args[1], args[2])
}

fn raw(ctx: &Context, args: &str) -> Result<()> {
    ctx.irc_send_raw(args)
}

fn join(ctx: &Context, args: &str) -> Result<()> {
    let cfg_id = {
        match ctx.source {
            IRC { ref config, .. } => config.clone(),
            _ => return Err(Error::new("must use this command from IRC")),
        }
    };
    {
        let db = ctx.bot.sql().lock();
        db.execute(
            "INSERT INTO irc_channels (channel, config_id) VALUES (?, ?) ON CONFLICT (channel, config_id) DO NOTHING",
            vec![args, cfg_id.as_str()],
        )?;
    }
    ctx.irc_send_raw(&format!("JOIN {}", args))?;
    ctx.say("done")
}

fn part(ctx: &Context, args: &str) -> Result<()> {
    let cfg_id = {
        match ctx.source {
            IRC { ref config, .. } => config.clone(),
            _ => return Err(Error::new("must use this command from IRC")),
        }
    };
    {
        let db = ctx.bot.sql().lock();
        db.execute(
            "DELETE FROM irc_channels WHERE channel = ? AND config_id = ?",
            vec![args, cfg_id.as_str()],
        )?;
    }
    ctx.irc_send_raw(&format!("part {}", args))?;
    ctx.say("done")
}

fn query(ctx: &Context, args: &str) -> Result<()> {
    let result: String = {
        let db = ctx.bot.sql().lock();
        let r = db.prepare(args).and_then(|mut stmt| {
            if stmt.column_count() == 0 {
                db.execute(args, NO_PARAMS).map(|n| format!("{} row(s) changed", n))
            } else {
                let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                let colstr = format!("({})", cols.join(", "));
                stmt.query_map(NO_PARAMS, |row| {
                    let vals: Vec<String> = (0..row.column_count())
                        .map(|i| match row.get_raw(i) {
                            Null => "null".to_string(),
                            Integer(i) => format!("{}", i),
                            Real(f) => format!("{}", f),
                            Text(s) => format!("{:?}", s),
                            Blob(b) => format!("{:?}", b),
                        })
                        .collect();
                    format!("({})", vals.join(", "))
                })
                .and_then(|rows| {
                    let r: std::result::Result<Vec<String>, rusqlite::Error> = rows.collect();
                    Ok(format!("{}: {}", colstr, r?.join(", ")))
                })
            }
        });
        r?
    };
    ctx.say(result.as_str())
}

fn whoami(ctx: &Context, _: &str) -> Result<()> {
    match ctx.source {
        IRC {
            ref config, ref prefix, ..
        } => {
            if let Some(p) = prefix {
                ctx.reply(Message::Simple(format!(
                    "You are {}:{}\nFlags: {}",
                    config,
                    p,
                    ctx.perms()?
                )))?;
            }
        }
        Discord { guild, ref user, .. } => ctx.reply(Message::Simple(format!(
            "You are {:?}:{}\nFlags: {}",
            guild.map(|g| *g.as_u64()),
            user.id.as_u64(),
            ctx.perms()?
        )))?,
    }
    Ok(())
}
