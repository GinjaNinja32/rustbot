extern crate shared;

use shared::types;
use std::collections::HashMap;

#[no_mangle]
pub extern "C" fn get_meta() -> types::Meta {
    let mut commands: HashMap<String, types::Command> = HashMap::new();
    commands.insert("drop".to_string(), |bot, channel, args| {
        for m in args.split(" ") {
            println!("dropping {}", m);
            bot.drop_module(m);
        }
        bot.send_privmsg(channel, "done");
    });
    commands.insert("load".to_string(), |bot, channel, args| {
        for m in args.split(" ") {
            println!("loading {}", m);
            bot.load_module(m);
        }
        bot.send_privmsg(channel, "done");
    });
    return types::Meta { commands: commands };
}
