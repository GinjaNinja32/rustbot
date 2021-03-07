#[macro_use]
extern crate nom;

mod dice;
mod swrpg;

use rustbot::prelude::*;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("dice", Command::new(cmd_dice));
    meta.cmd("swrpg", Command::new(cmd_swrpg));
}

fn cmd_dice(ctx: &dyn Context, args: &str) -> Result<()> {
    let v = dice::parse(args).map_err(UserError::new)?;
    let limit = dice::limits::Limiter::new(10000);
    let result = dice::eval(&v, limit).map_err(UserError::new)?;
    ctx.reply(Message::Spans(result))
}

fn cmd_swrpg(ctx: &dyn Context, args: &str) -> Result<()> {
    let result = swrpg::parse_and_eval(args).map_err(UserError::new)?;
    ctx.reply(Message::Spans(result))
}
