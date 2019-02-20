#[macro_use]
extern crate nom;
extern crate rand;
extern crate shared;

mod dice;

use shared::types;
use std::collections::BTreeMap;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut commands: BTreeMap<String, types::Command> = BTreeMap::new();
    commands.insert("dice".to_string(), cmd_dice);
    types::Meta { commands }
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
