use libloading::{Library, Symbol};
use irc::client::prelude::*;
use std::rc::Rc;
use std::collections::HashMap;
use shared::types;
use config;

struct Bot {
    client: Rc<IrcClient>,
    conf: config::Config,
    modules: HashMap<String, Module>,
    commands: HashMap<String, types::Command>,
}

impl Bot {
    fn incoming(&mut self, irc_msg: Message) {
        if let Command::PRIVMSG(channel, message) = irc_msg.command {
            if message.is_empty() {
                return
            }

            if self.conf.cmdchars.contains(message.get(0..1).unwrap()) {
                // it's a command!
                let parts: Vec<&str> = message.get(1..).unwrap().splitn(2, " ").collect();
                match self.commands.get(parts[0]).map(|&c| c) {
                    Some(f) => f(self, channel.as_str(), parts.get(1).unwrap_or(&"")),
                    None => ()
                }
            }
        }
    }
}

impl types::Bot for Bot {
    fn send_privmsg(&self, chan: &str, msg: &str) {
        self.client.send_privmsg(chan, msg).unwrap();
    }

    fn load_module(&mut self, name: &str) {
        let lib = Library::new(format!("libmod_{}.so", name)).unwrap();

        let m = Module{
            name: name.to_string(),
            lib: lib,
        };
        let meta = m.get_meta();
        for command in meta.commands.iter() {
            self.commands.insert(command.0.to_string(), *command.1);
        }
        self.modules.insert(name.to_string(), m);
    }
}

pub fn start() {
    let conf = config::load_config();
    println!("{:?}", conf);

    let client = Rc::new(IrcClient::new("conf/irc.toml").unwrap());
    let b = &mut Bot{
        client: Rc::clone(&client),
        conf: conf,
        modules: HashMap::new(),
        commands: HashMap::new(),
    };
    client.send_cap_req(&[Capability::MultiPrefix]).unwrap();
    client.identify().unwrap();
    types::Bot::load_module(b, "admin"); // WHY
    client.for_each_incoming(|irc_msg| { b.incoming(irc_msg) }).unwrap();
}

struct Module {
    name: String,
    lib: Library,
}

impl Module {
    fn get_meta(&self) -> types::Meta {
        unsafe {
            let func: Symbol<fn() -> types::Meta> = match self.lib.get(b"get_meta") {
                Ok(func) => func,
                Err(err) => {
                    println!("{}: failed to find get_meta function: {}", self.name, err);
                    return types::Meta{
                        commands: HashMap::new(),
                    };
                }
            };

            return func()
        }
    }
}
