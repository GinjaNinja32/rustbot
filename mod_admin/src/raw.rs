use rustbot::prelude::*;

pub fn dmsg(ctx: &dyn Context, args: &str) -> Result<()> {
    let mut args: Vec<&str> = args.splitn(4, ' ').collect();
    if args.len() != 4 {
        return Err("usage: dmsg <config_id> <guild> <channel> <message...>".into());
    }

    if args[2].chars().collect::<Vec<char>>()[0] == '#' {
        args[2] = &args[2][1..];
    }

    ctx.bot().dis_send_message(args[0], args[1], args[2], args[3], true)
}

pub fn imsg(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, ' ').collect();
    if args.len() != 3 {
        return Err("usage: imsg <config_id> <channel> <message...>".into());
    }

    ctx.bot().irc_send_privmsg(args[0], args[1], args[2])
}

pub fn raw(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(2, ' ').collect();
    if args.len() != 2 {
        return Err("usage: raw <config_id> <message...>".into());
    }

    ctx.bot().irc_send_raw(args[0], args[1])
}

pub fn join(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, ' ').collect();
    if args.len() != 2 {
        return Err("usage: join <config_id> <channel>".into());
    }

    {
        let mut db = ctx.bot().sql().lock();
        db.execute(
            "INSERT INTO irc_channels (config_id, channel) VALUES ($1, $2) ON CONFLICT (config_id, channel) DO NOTHING",
            &[&args[0], &args[1]],
        )?;
    }
    ctx.bot().irc_send_raw(args[0], &format!("JOIN {}", args[1]))?;
    ctx.say("done")
}

pub fn part(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, ' ').collect();
    if args.len() != 2 {
        return Err("usage: part <config_id> <channel>".into());
    }
    {
        let mut db = ctx.bot().sql().lock();
        db.execute(
            "DELETE FROM irc_channels WHERE channel = $1 AND config_id = $2",
            &[&args[0], &args[1]],
        )?;
    }
    ctx.bot().irc_send_raw(args[0], &format!("part {}", args[1]))?;
    ctx.say("done")
}
