extern crate rusqlite;
extern crate shared;

use rusqlite::types::ValueRef::*;
use rusqlite::NO_PARAMS;
use shared::types;
use shared::types::Source::*;
use std::sync::Arc;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut meta = types::Meta::new();
    meta.commandrc("raw", Arc::new(wrap(raw)));
    meta.commandrc("join", Arc::new(wrap(join)));
    meta.commandrc("part", Arc::new(wrap(part)));
    meta.commandrc("e", Arc::new(wrap(exec)));
    meta.commandrc("q", Arc::new(wrap(query)));
    meta.command("whoami", whoami);
    meta
}

fn wrap(f: impl Fn(&mut types::Context, &str)) -> impl Fn(&mut types::Context, &str) {
    move |ctx: &mut types::Context, args| {
        if ctx.has_perm(types::PERM_ADMIN) {
            f(ctx, args)
        } else {
            ctx.reply("permission denied")
        }
    }
}

fn raw(ctx: &mut types::Context, args: &str) {
    ctx.irc_send_raw(args);
}

fn join(ctx: &mut types::Context, args: &str) {
    let cfg_id = {
        match ctx.get_source() {
            Some(IRCUser { config, .. }) => config,
            Some(IRCServer { config, .. }) => config,
            _ => return,
        }
    };
    let result = {
        let db = ctx.bot().sql().lock().unwrap();
        let r = db.execute(
            "INSERT INTO irc_channels (channel, config_id) VALUES (?, ?) ON CONFLICT (channel, config_id) DO NOTHING",
            vec![args, cfg_id.as_str()],
        );
        r
    };
    if let Err(e) = result {
        ctx.reply(&format!("join failed: {}", e));
        return;
    }
    ctx.irc_send_raw(&format!("JOIN {}", args));
    ctx.reply("done");
}

fn part(ctx: &mut types::Context, args: &str) {
    let cfg_id = {
        match ctx.get_source() {
            Some(IRCUser { config, .. }) => config,
            Some(IRCServer { config, .. }) => config,
            _ => return,
        }
    };
    let result = {
        let db = ctx.bot().sql().lock().unwrap();
        let r = db.execute(
            "DELETE FROM irc_channels WHERE channel = ? AND config_id = ?",
            vec![args, cfg_id.as_str()],
        );
        r
    };
    if let Err(e) = result {
        ctx.reply(&format!("part failed: {}", e));
        return;
    }
    ctx.irc_send_raw(&format!("part {}", args));
    ctx.reply("done");
}

fn exec(ctx: &mut types::Context, args: &str) {
    let result = {
        let db = ctx.bot().sql().lock().unwrap();
        let r = db.execute(args, NO_PARAMS);
        r
    };
    match result {
        Ok(n) => ctx.reply(&format!("{} rows changed", n)),
        Err(e) => ctx.reply(&format!("{}", e)),
    }
}

fn query(ctx: &mut types::Context, args: &str) {
    let result: Result<(String, Vec<String>), rusqlite::Error> = {
        let db = ctx.bot().sql().lock().unwrap();
        let r = db.prepare(args).and_then(|mut stmt| {
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
                let r: Result<Vec<String>, rusqlite::Error> = rows.collect();
                Ok((colstr, r?))
            })
        });
        r
    };
    match result {
        Ok((cols, rows)) => ctx.reply(&format!("{}: {}", cols, &rows.join(", "))),
        Err(e) => ctx.reply(&format!("{}", e)),
    }
}

fn whoami(ctx: &mut types::Context, _: &str) {
    match ctx.get_source() {
        None => ctx.reply(&format!("I don't know who you are")),
        Some(IRCServer { config, host, .. }) => ctx.reply(&format!("You are {}:{}", config, host)),
        Some(IRCUser {
            config,
            nick,
            user,
            host,
            ..
        }) => ctx.reply(&format!("You are {}:{}!{}@{}", config, nick, user, host)),
        Some(DiscordUser { guild, user, .. }) => ctx.reply(&format!(
            "You are {:?}:{}",
            guild.map(|g| *g.as_u64()),
            user.id.as_u64(),
        )),
    }
    ctx.reply(&format!("Flags: {}", ctx.perms()));
}
