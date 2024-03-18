use lazy_static::lazy_static;
use regex::Regex;
use rustbot::prelude::*;
use std::borrow::Cow;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("isbridge", Command::new(isbridge).req_perms(Perms::Admin));

    meta.handle(HandleType::All, Box::new(do_debridge));
}

fn isbridge(ctx: &dyn Context, args: &str) -> Result<()> {
    let mut db = ctx.bot().sql().lock();

    let args: Vec<&str> = args.splitn(3, char::is_whitespace).collect();

    let config = args[0];
    let user = *args.get(1).unwrap_or(&"");
    let spec = *args.get(2).unwrap_or(&"");

    if user == "" {
        return ctx.reply(Message::Simple("todo".to_string()));
    }

    if spec == "" {
        db.execute(
            "DELETE FROM mod_debridge WHERE config_id = $1 AND source_user = $2",
            &[&config, &user],
        )?;
    } else {
        db.execute(
            "INSERT INTO mod_debridge (config_id, source_user, spec) VALUES ($1, $2, $3) ON CONFLICT (config_id, source_user) DO UPDATE SET spec = $3",
            &[&config, &user, &spec],
        )?;
    }

    ctx.reply(Message::Simple("done".to_string()))
}

lazy_static! {
    static ref DEBRIDGE_RE: Regex = Regex::new(r"<([^.]+)> (.*)").unwrap();
}

fn do_debridge(ctx: &dyn Context, _typ: HandleType, msg: &str) -> Result<()> {
    let user = ctx.source().user_string();

    let spec: Option<String> = {
        let mut db = ctx.bot().sql().lock();

        db.query_opt(
            "SELECT spec FROM mod_debridge WHERE config_id = $1 AND $2 LIKE source_user",
            &[&ctx.config_id(), &user],
        )?
        .map(|row| row.get(0))
    };

    if spec.is_some() {
        // TODO do something with the spec value
        if let Some(cap) = DEBRIDGE_RE.captures(msg) {
            ctx.do_sub(&cap[1], &cap[2])?;
        }
    }

    Ok(())
}
