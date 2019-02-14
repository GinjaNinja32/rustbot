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

fn drop(ctx: &mut types::Context, args: &str) {
    for m in args.split(' ') {
        println!("dropping {}", m);
        ctx.bot().drop_module(m);
    }
    ctx.reply("done");
}

fn load(ctx: &mut types::Context, args: &str) {
    for m in args.split(' ') {
        println!("loading {}", m);
        ctx.bot().load_module(m);
    }
    ctx.reply("done");
}

fn reload(ctx: &mut types::Context, args: &str) {
    for m in args.split(' ') {
        println!("reloading {}", m);
        ctx.bot().drop_module(m);
        ctx.bot().load_module(m);
    }
    ctx.reply("done");
}
