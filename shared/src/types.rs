use std::collections::HashMap;

pub type Command = fn(&mut Bot, channel: &str, args: &str);

pub struct Meta {
    pub commands: HashMap<String, Command>,
}

pub trait Bot {
    fn send_privmsg(&self, &str, &str);
    fn load_module(&mut self, &str);
}
