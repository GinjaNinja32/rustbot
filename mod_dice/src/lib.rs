#[macro_use]
extern crate nom;
extern crate rand;
extern crate rustbot;

mod dice;

use rustbot::prelude::*;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("dice", Command::new(cmd_dice));
}

fn cmd_dice(ctx: &Context, args: &str) -> Result<()> {
    let v = dice::parse(args)?;
    let result = dice::eval(v)?;
    ctx.say(result.as_str())
}
