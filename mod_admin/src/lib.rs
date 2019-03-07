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
    meta.commandrc("e", Rc::new(wrap(exec)));
    meta.commandrc("q", Rc::new(wrap(query)));
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

fn exec(ctx: &mut types::Context, args: &str) {
    match ctx.bot().sql().execute(args, NO_PARAMS) {
        Ok(n) => ctx.reply(&format!("{} rows changed", n)),
        Err(e) => ctx.reply(&format!("{}", e)),
    }
}

fn query(ctx: &mut types::Context, args: &str) {
    let result: Result<Vec<String>, rusqlite::Error> =
        ctx.bot().sql().prepare(args).and_then(|mut stmt| {
            stmt.query_map(NO_PARAMS, |row| {
                let cols: Vec<String> = (0..row.column_count())
                    .map(|i| match row.get_raw(i) {
                        Null => "null".to_string(),
                        Integer(i) => format!("{}", i),
                        Real(f) => format!("{}", f),
                        Text(s) => format!("{:?}", s),
                        Blob(b) => format!("{:?}", b),
                    })
                    .collect();
                format!("[{}]", cols.join(", "))
            })
            .and_then(|rows| rows.collect())
        });
    match result {
        Ok(rows) => ctx.reply(&rows.join(", ")),
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
    ctx.reply(&format!("Admin: {}", ctx.has_perm("admin")));
}
