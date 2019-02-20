#[macro_use]
extern crate nom;
extern crate rand;
extern crate shared;

mod dice;

use shared::types;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut meta = types::Meta::new();
    meta.command("dice", cmd_dice);
    meta
}

fn cmd_dice(ctx: &mut types::Context, args: &str) {
    match dice::parse(args) {
        Ok(v) => match dice::eval(v) {
            Ok(result) => ctx.reply(result.as_str()),
            Err(v) => ctx.reply(v.as_str()),
        },
        Err(v) => ctx.reply(v.as_str()),
    }
}
