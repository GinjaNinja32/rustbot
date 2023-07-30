mod dice;
mod swrpg;

use rustbot::prelude::*;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("dice", Command::new(cmd_dice));
    meta.cmd("swrpg", Command::new(cmd_swrpg));
    meta.cmd("space", Command::new(cmd_space));
}

fn cmd_dice(ctx: &dyn Context, args: &str) -> Result<()> {
    if args.trim().is_empty() {
        return ctx.reply(Message::Simple(
            "Usage: dice <roll>; try '1d6', '2d20H1', '2d6>7'".to_string(),
        ));
    }
    let v = dice::Command::new(args).map_err(UserError::new)?;
    let mut limit = dice::limits::Limiter::new(10000);
    let result = v.eval(&mut limit).map_err(UserError::new)?;
    ctx.reply(Message::Spans(result))
}

fn cmd_swrpg(ctx: &dyn Context, args: &str) -> Result<()> {
    let result = swrpg::parse_and_eval(args).map_err(UserError::new)?;
    ctx.reply(Message::Spans(result))
}

fn cmd_space(ctx: &dyn Context, args: &str) -> Result<()> {
    if args.trim().is_empty() {
        return ctx.reply(Message::Simple("Usage: space <dice> [<description>...]".to_string()));
    }

    let (count, desc) = match args.find(' ') {
        None => (args, String::new()),
        Some(idx) => {
            let (count, desc) = args.split_at(idx);
            (count, format!("{desc}: "))
        }
    };

    let expr = format!(
        "D:{count}; R:$Dd6; C:s($Re=6); O:s($Re=1); S:s($Re>=5); {desc}$R: $S success%[es], $C six%[es], $O one%s",
    );

    cmd_dice(ctx, &expr)
}
