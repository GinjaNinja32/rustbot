extern crate shared;

use std::collections::HashMap;
use shared::types;

#[no_mangle]
pub extern "C" fn get_meta() -> types::Meta {
    let mut commands: HashMap<String, types::Command> = HashMap::new();
    commands.insert("test".to_string(), |bot, channel, args| {
        println!("test command with args = {:?}", args);
        bot.send_privmsg(channel, "beep boop");
    });
    return types::Meta{
        commands: commands,
    }
}
