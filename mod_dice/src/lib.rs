#[macro_use]
extern crate nom;

mod dice;
mod swrpg;

use rustbot::prelude::*;
use rustbot::{spans, spans_plural};

use dice::Evaluable;

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
    let v = dice::parse(args).map_err(UserError::new)?;
    let limit = dice::limits::Limiter::new(10000);
    let result = dice::eval(&v, limit).map_err(UserError::new)?;
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
        None => (args, spans! {}),
        Some(idx) => {
            let (count, desc) = args.split_at(idx);
            (count, spans! {desc, ": "})
        }
    };

    let expr = format!("{}d6", count);

    let expr = dice::parse(&expr).map_err(UserError::new)?;
    let mut limit = dice::limits::Limiter::new(10000);

    let (_, v) = expr.eval(&mut limit).map_err(UserError::new)?;
    let v = v.to_int_slice().map_err(UserError::new)?;

    let ones = v.iter().filter(|v| **v == 1).count();
    let sixes = v.iter().filter(|v| **v == 6).count();
    let successes = v.iter().filter(|v| **v >= 5).count();

    return ctx.reply(Message::Spans(spans! {
        desc,
        format!("{:?}", v), ": ",
        spans_plural!(successes, "success", "es"), ", ",
        spans_plural!(sixes, "six", "es"), ", ",
        spans_plural!(ones, "one"),
    }));
}
