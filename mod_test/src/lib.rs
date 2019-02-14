extern crate shared;

use shared::types;
use std::collections::BTreeMap;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut commands: BTreeMap<String, types::Command> = BTreeMap::new();
    commands.insert("test".to_string(), |ctx, _args| {
        ctx.reply("beep boop");
    });
    types::Meta { commands }
}
