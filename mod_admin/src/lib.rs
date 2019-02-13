extern crate shared;

use std::collections::HashMap;
use shared::types;

#[no_mangle]
pub extern "C" fn get_meta() -> types::Meta {
    let mut commands: HashMap<String, types::Command> = HashMap::new();
    commands.insert("load".to_string(), |bot, channel, args| {
        for m in args.split(" ") {
            bot.load_module(m);
        }
        bot.send_privmsg(channel, "done");
    });
    return types::Meta{
        commands: commands,
    }
}
