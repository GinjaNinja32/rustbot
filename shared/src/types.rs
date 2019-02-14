use std::collections::HashMap;

pub type Command = fn(&mut Bot, &Context, args: &str);

pub struct Meta {
    pub commands: HashMap<String, Command>,
}

pub trait Context {
    fn reply(&self, &Bot, &str);
}

pub trait Bot {
    fn send_privmsg(&self, &str, &str);
    fn load_module(&mut self, &str);
    fn drop_module(&mut self, &str);
}
