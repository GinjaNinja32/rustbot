extern crate shared;

use shared::types;
use std::collections::HashMap;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut commands: HashMap<String, types::Command> = HashMap::new();
    commands.insert("drop".to_string(), drop);
    commands.insert("load".to_string(), load);
    commands.insert("reload".to_string(), reload);
    types::Meta { commands }
}

fn drop(bot: &mut types::Bot, ctx: &types::Context, args: &str) {
    for m in args.split(' ') {
        println!("dropping {}", m);
        bot.drop_module(m);
    }
    ctx.reply(bot, "done");
}

fn load(bot: &mut types::Bot, ctx: &types::Context, args: &str) {
    for m in args.split(' ') {
        println!("loading {}", m);
        bot.load_module(m);
    }
    ctx.reply(bot, "done");
}

fn reload(bot: &mut types::Bot, ctx: &types::Context, args: &str) {
    for m in args.split(' ') {
        println!("reloading {}", m);
        bot.drop_module(m);
        bot.load_module(m);
    }
    ctx.reply(bot, "done");
}
