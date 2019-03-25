extern crate shared;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.cmd("8ball", Command::new(|ctx, args| randomlist("eightball", ctx, args)));
    meta.cmd("kitty", Command::new(|ctx, args| randomlist("kitty", ctx, args)));
    meta.cmd("fox", Command::new(|ctx, args| randomlist("fox", ctx, args)));
    meta.cmd("snek", Command::new(|ctx, args| randomlist("snek", ctx, args)));
    meta.cmd("delrand", Command::new(delrand).req_perms(Perms::Admin));
    meta
}

fn delrand(ctx: &Context, args: &str) -> Result<()> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return ctx.say("usage: delrand <category> <string>");
    }

    let n = ctx.bot.sql().lock().execute(
        "DELETE FROM mod_randomlist WHERE category = ? AND string = ?",
        vec![parts[0], parts[1]],
    )?;
    if n != 1 {
        ctx.say(&format!("{} rows removed", n))
    } else {
        ctx.say(&format!("done"))
    }
}

fn randomlist(what: &str, ctx: &Context, args: &str) -> Result<()> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() == 2 && parts[0] == "add" {
        let n = ctx.bot.sql().lock().execute(
            "INSERT INTO mod_randomlist (category, string) VALUES (?, ?) ON CONFLICT (category, string) DO NOTHING",
            vec![what, parts[1]],
        )?;
        if n == 0 {
            return ctx.say("That's already on the list.");
        }
        return ctx.say("Added.");
    }

    let s: std::result::Result<Vec<String>, rusqlite::Error> = {
        let db = ctx.bot.sql().lock();
        let r = db
            .prepare("SELECT string FROM mod_randomlist WHERE category = ? LIMIT ( ABS(RANDOM()) % MAX((SELECT COUNT(*) FROM mod_randomlist WHERE category = ?), 1) ), 1").and_then(|mut stmt| {
                stmt.query_map(vec![what, what], |row| row.get(0))?
                .collect()
            });
        r
    };

    match s?.get(0) {
        Some(s) => ctx.say(s)?,
        None => ctx.say(&format!("I don't have anything to give you for '{}'", what))?,
    }

    Ok(())
}
