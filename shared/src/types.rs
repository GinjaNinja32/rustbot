use std::collections::BTreeMap;

pub type Command = fn(&mut Context, args: &str);

pub struct Meta {
    pub commands: BTreeMap<String, Command>,
}

pub trait Context {
    fn reply(&self, &str);
    fn get_source(&self) -> Option<Source>;
    fn bot(&mut self) -> &mut Bot;
}

pub trait Bot {
    fn send_privmsg(&self, &str, &str);
    fn load_module(&mut self, &str);
    fn drop_module(&mut self, &str);
}

#[derive(Debug, Clone)]
pub enum Source {
    Server(String),
    User {
        nick: String,
        user: String,
        host: String,
    },
}
