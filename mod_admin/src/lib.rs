extern crate rusqlite;
extern crate shared;

use rusqlite::types::ValueRef::*;
use rusqlite::NO_PARAMS;
use shared::prelude::*;
use std::sync::Arc;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.commandrc("raw", Arc::new(wrap(raw)));
    meta.commandrc("join", Arc::new(wrap(join)));
    meta.commandrc("part", Arc::new(wrap(part)));
    meta.commandrc("q", Arc::new(wrap(query)));
    meta.command("whoami", whoami);
    meta
}

fn wrap(f: impl Fn(&mut Context, &str) -> Result<()>) -> impl Fn(&mut Context, &str) -> Result<()> {
    move |ctx: &mut Context, args| {
        if ctx.has_perm(Perms::Admin)? {
            f(ctx, args)
        } else {
            ctx.reply("permission denied")
        }
    }
}

fn raw(ctx: &mut Context, args: &str) -> Result<()> {
    ctx.irc_send_raw(args)
}

fn join(ctx: &mut Context, args: &str) -> Result<()> {
    let cfg_id = {
        match ctx.source {
            IRC { ref config, .. } => config.clone(),
            _ => return Err(Error::new("must use this command from IRC")),
        }
    };
    {
        let db = ctx.bot.sql().lock().unwrap();
        db.execute(
            "INSERT INTO irc_channels (channel, config_id) VALUES (?, ?) ON CONFLICT (channel, config_id) DO NOTHING",
            vec![args, cfg_id.as_str()],
        )?;
    }
    ctx.irc_send_raw(&format!("JOIN {}", args))?;
    ctx.reply("done")
}

fn part(ctx: &mut Context, args: &str) -> Result<()> {
    let cfg_id = {
        match ctx.source {
            IRC { ref config, .. } => config.clone(),
            _ => return Err(Error::new("must use this command from IRC")),
        }
    };
    {
        let db = ctx.bot.sql().lock().unwrap();
        db.execute(
            "DELETE FROM irc_channels WHERE channel = ? AND config_id = ?",
            vec![args, cfg_id.as_str()],
        )?;
    }
    ctx.irc_send_raw(&format!("part {}", args))?;
    ctx.reply("done")
}

fn query(ctx: &mut Context, args: &str) -> Result<()> {
    let result: String = {
        let db = ctx.bot.sql().lock()?;
        let r = db.prepare(args).and_then(|mut stmt| {
            if stmt.column_count() == 0 {
                db.execute(args, NO_PARAMS)
                    .map(|n| format!("{} row(s) changed", n))
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
    ctx.reply(result.as_str())
}

fn whoami(ctx: &mut Context, _: &str) -> Result<()> {
    match ctx.source {
        IRC {
            ref config,
            ref prefix,
            ..
        } => {
            if let Some(p) = prefix {
                ctx.reply(&format!("You are {}:{}", config, p))?;
            }
        }
        Discord {
            guild, ref user, ..
        } => ctx.reply(&format!(
            "You are {:?}:{}",
            guild.map(|g| *g.as_u64()),
            user.id.as_u64(),
        ))?,
    }
    ctx.reply(&format!("Flags: {}", ctx.perms()?))
}
