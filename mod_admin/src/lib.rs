extern crate rusqlite;
extern crate shared;

use rusqlite::types::ValueRef::*;
use rusqlite::NO_PARAMS;
use shared::types;
use shared::types::Source::*;
use std::rc::Rc;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut meta = types::Meta::new();
    meta.commandrc("raw", Rc::new(wrap(raw)));
    meta.commandrc("join", Rc::new(wrap(join)));
    meta.commandrc("part", Rc::new(wrap(part)));
    meta.commandrc("e", Rc::new(wrap(exec)));
    meta.commandrc("q", Rc::new(wrap(query)));
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
    ctx.bot().send_raw(args);
}

fn join(ctx: &mut types::Context, args: &str) {
    if let Err(e) = ctx.bot().sql().execute("INSERT INTO channels (channel) VALUES (?) ON CONFLICT (channel) DO NOTHING", vec![args]) {
        ctx.reply(&format!("join failed: {}", e));
        return;
    }
    ctx.bot().send_raw(&format!("JOIN {}", args));
    ctx.reply("done");
}

fn part(ctx: &mut types::Context, args: &str) {
    if let Err(e) = ctx.bot().sql().execute("DELETE FROM channels WHERE channel = ?", vec![args]) {
        ctx.reply(&format!("part failed: {}", e));
        return;
    }
    ctx.bot().send_raw(&format!("part {}", args));
    ctx.reply("done");
}

fn exec(ctx: &mut types::Context, args: &str) {
    match ctx.bot().sql().execute(args, NO_PARAMS) {
        Ok(n) => ctx.reply(&format!("{} rows changed", n)),
        Err(e) => ctx.reply(&format!("{}", e)),
    }
}

fn query(ctx: &mut types::Context, args: &str) {
    let result: Result<(String, Vec<String>), rusqlite::Error> =
        ctx.bot().sql().prepare(args).and_then(|mut stmt| {
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
    match result {
        Ok((cols, rows)) => ctx.reply(&format!("{}: {}", cols, &rows.join(", "))),
        Err(e) => ctx.reply(&format!("{}", e)),
    }
}

fn whoami(ctx: &mut types::Context, _: &str) {
    match ctx.get_source() {
        None => ctx.reply(&format!("I don't know who you are")),
        Some(Server(s)) => ctx.reply(&format!("You are {}", s)),
        Some(User { nick, user, host }) => {
            ctx.reply(&format!("You are {}!{}@{}", nick, user, host))
        }
    }
    ctx.reply(&format!("Flags: {}", ctx.perms()));
}
