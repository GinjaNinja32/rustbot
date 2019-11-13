use rustbot::prelude::*;

pub fn dmsg(ctx: &Context, args: &str) -> Result<()> {
    let mut args: Vec<&str> = args.splitn(3, " ").collect();
    if args.len() != 3 {
        return Err(Error::new("usage: dmsg <config_id> <channel> <message...>"));
    }

    if args[1].chars().collect::<Vec<char>>()[0] == '#' {
        args[1] = &args[1][1..];
    }

    ctx.bot.dis_send_message(args[0], args[1], args[2], true)
}

pub fn imsg(ctx: &Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, " ").collect();
    if args.len() != 3 {
        return Err(Error::new("usage: imsg <config_id> <channel> <message...>"));
    }

    ctx.bot.irc_send_privmsg(args[0], args[1], args[2])
}

pub fn raw(ctx: &Context, args: &str) -> Result<()> {
    ctx.irc_send_raw(args)
}

pub fn join(ctx: &Context, args: &str) -> Result<()> {
    let cfg_id = {
        match ctx.source {
            IRC { ref config, .. } => config.clone(),
            _ => return Err(Error::new("must use this command from IRC")),
        }
    };
    {
        let db = ctx.bot.sql().lock();
        db.execute(
            "INSERT INTO irc_channels (channel, config_id) VALUES ($1, $2) ON CONFLICT (channel, config_id) DO NOTHING",
            &[&args, &cfg_id],
        )?;
    }
    ctx.irc_send_raw(&format!("JOIN {}", args))?;
    ctx.say("done")
}

pub fn part(ctx: &Context, args: &str) -> Result<()> {
    let cfg_id = {
        match ctx.source {
            IRC { ref config, .. } => config.clone(),
            _ => return Err(Error::new("must use this command from IRC")),
        }
    };
    {
        let db = ctx.bot.sql().lock();
        db.execute(
            "DELETE FROM irc_channels WHERE channel = $1 AND config_id = $2",
            &[&args, &cfg_id],
        )?;
    }
    ctx.irc_send_raw(&format!("part {}", args))?;
    ctx.say("done")
}
