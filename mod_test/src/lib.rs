extern crate shared;

use shared::types;
use std::collections::HashMap;

#[no_mangle]
pub extern "C" fn get_meta() -> types::Meta {
    let mut commands: HashMap<String, types::Command> = HashMap::new();
    commands.insert("test".to_string(), |bot, channel, args| {
        println!("test command with args = {:?}", args);
        bot.send_privmsg(channel, "beep boop");
    });
    types::Meta { commands }
}
