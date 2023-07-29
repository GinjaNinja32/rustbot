use rustbot::prelude::*;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("8ball", Command::new(|ctx, args| randomlist("eightball", ctx, args)));
    meta.cmd("kitty", Command::new(|ctx, args| randomlist("kitty", ctx, args)));
    meta.cmd("fox", Command::new(|ctx, args| randomlist("fox", ctx, args)));
    meta.cmd("snek", Command::new(|ctx, args| randomlist("snek", ctx, args)));
    meta.cmd("otter", Command::new(|ctx, args| randomlist("otter", ctx, args)));
    meta.cmd("doggo", Command::new(|ctx, args| randomlist("doggo", ctx, args)));
    meta.cmd("possum", Command::new(|ctx, args| randomlist("possum", ctx, args)));
    meta.cmd("lizard", Command::new(|ctx, args| randomlist("lizard", ctx, args)));
    meta.cmd("delrand", Command::new(delrand).req_perms(Perms::Admin));
}

fn delrand(ctx: &dyn Context, args: &str) -> Result<()> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return ctx.say("usage: delrand <category> <string>");
    }

    let n = ctx.bot().sql().lock().execute(
        "DELETE FROM mod_randomlist WHERE category = $1 AND string = $2",
        &[&parts[0], &parts[1]],
    )?;
    if n != 1 {
        ctx.say(&format!("{n} rows removed"))
    } else {
        ctx.say("done")
    }
}

fn randomlist(what: &str, ctx: &dyn Context, args: &str) -> Result<()> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() == 2 && parts[0] == "add" {
        let n = ctx.bot().sql().lock().execute(
            "INSERT INTO mod_randomlist (category, string) VALUES ($1, $2) ON CONFLICT (category, string) DO NOTHING",
            &[&what, &parts[1]],
        )?;
        if n == 0 {
            return ctx.say("That's already on the list.");
        }
        return ctx.say("Added.");
    }

    let mut db = ctx.bot().sql().lock();
    let rows = db
            .query("SELECT string FROM mod_randomlist WHERE category = $1 LIMIT 1 OFFSET FLOOR(RANDOM() * GREATEST((SELECT COUNT(*) FROM mod_randomlist WHERE category = $1), 1) )", &[&what])?;
    if rows.is_empty() {
        return ctx.say(&format!("I don't have anything to give you for '{what}'"));
    }

    let s: String = rows.get(0).unwrap().get(0);
    ctx.say(&s)
}
