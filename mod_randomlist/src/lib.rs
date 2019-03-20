extern crate shared;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.command("8ball", |ctx, args| randomlist("eightball", ctx, args));
    meta.command("kitty", |ctx, args| randomlist("kitty", ctx, args));
    meta.command("fox", |ctx, args| randomlist("fox", ctx, args));
    meta.command("snek", |ctx, args| randomlist("snek", ctx, args));
    meta.command("delrand", delrand);
    meta
}

fn delrand(ctx: &mut Context, args: &str) -> Result<()> {
    if !ctx.has_perm(PERM_ADMIN)? {
        return ctx.reply("permission denied");
    }

    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return ctx.reply("usage: delrand <category> <string>");
    }

    let n = ctx.bot.sql().lock()?.execute(
        "DELETE FROM mod_randomlist WHERE category = ? AND string = ?",
        vec![parts[0], parts[1]],
    )?;
    if n != 1 {
        ctx.reply(&format!("{} rows removed", n))
    } else {
        ctx.reply(&format!("done"))
    }
}

fn randomlist(what: &str, ctx: &mut Context, args: &str) -> Result<()> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() == 2 && parts[0] == "add" {
        ctx.bot.sql().lock()?.execute(
            "INSERT INTO mod_randomlist (category, string) VALUES (?, ?)",
            vec![what, parts[1]],
        )?;
        return ctx.reply("Added");
    }

    let s: std::result::Result<Vec<String>, rusqlite::Error> = {
        let db = ctx.bot.sql().lock()?;
        let r = db
            .prepare("SELECT string FROM mod_randomlist WHERE category = ? LIMIT ( ABS(RANDOM()) % MAX((SELECT COUNT(*) FROM mod_randomlist WHERE category = ?), 1) ), 1").and_then(|mut stmt| {
                stmt.query_map(vec![what, what], |row| row.get(0))?
                .collect()
            });
        r
    };

    match s?.get(0) {
        Some(s) => ctx.reply(s)?,
        None => ctx.reply(&format!("I don't have anything to give you for '{}'", what))?,
    }

    Ok(())
}
