use rusqlite::Connection;
use std::collections::BTreeMap;
use std::rc::Rc;

pub type Command = Rc<Fn(&mut Context, &str)>;

pub struct Meta {
    commands: BTreeMap<String, Command>,
}

impl Meta {
    pub fn new() -> Meta {
        Meta {
            commands: BTreeMap::new(),
        }
    }
    pub fn command(&mut self, name: &str, f: fn(&mut Context, &str)) {
        self.commands.insert(name.to_string(), Rc::new(f));
    }
    pub fn commandrc(&mut self, name: &str, f: Command) {
        self.commands.insert(name.to_string(), f);
    }
    pub fn commands(&self) -> &BTreeMap<String, Command> {
        &self.commands
    }
}

pub trait Context {
    fn reply(&self, &str);
    fn has_perm(&self, &str) -> bool;
    fn get_source(&self) -> Option<Source>;
    fn bot(&mut self) -> &mut Bot;
}

pub trait Bot {
    fn send_privmsg(&self, &str, &str);
    fn load_module(&mut self, &str);
    fn drop_module(&mut self, &str);
    fn has_perm(&self, &str, &str) -> bool;
    fn send_raw(&mut self, &str);

    fn sql(&mut self) -> &Connection;
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
