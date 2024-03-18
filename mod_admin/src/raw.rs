use rustbot::prelude::*;

pub fn dmsg(ctx: &dyn Context, args: &str) -> Result<()> {
    parse_args! {args,
        config: Atom,
        guild: Atom,
        channel: Atom,
        message: Rest,
    }

    let mut channel: &str = &channel;
    if channel.chars().next().unwrap() == '#' {
        channel = &channel[1..];
    }

    ctx.bot().dis_send_message(&config, &guild, channel, &message, true)
}

pub fn imsg(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, ' ').collect();
    if args.len() != 3 {
        bail_user!("usage: imsg <config_id> <channel> <message...>")
    }

    ctx.bot().irc_send_privmsg(args[0], args[1], args[2])
}

pub fn raw(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(2, ' ').collect();
    if args.len() != 2 {
        bail_user!("usage: raw <config_id> <message...>")
    }

    ctx.bot().irc_send_raw(args[0], args[1])
}

pub fn join(ctx: &dyn Context, args: &str) -> Result<()> {
    let args: Vec<&str> = args.splitn(3, ' ').collect();
    if args.len() != 2 {
        bail_user!("usage: join <config_id> <channel>")
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
        bail_user!("usage: part <config_id> <channel>")
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
