use libloading::{Library, Symbol};
use irc::client::prelude::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::io;
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

            match message.get(0..1) {
                Some(c) => {
                    if self.conf.cmdchars.contains(c) {
                        // it's a command!
                        let parts: Vec<&str> = message[1..].splitn(2, " ").collect();
                        match self.commands.get(parts[0]).map(|&c| c) {
                            Some(f) => f(self, channel.as_str(), parts.get(1).unwrap_or(&"")),
                            None => ()
                        }
                    }
                },
                None => ()
            }
        }
    }
}

impl types::Bot for Bot {
    fn send_privmsg(&self, chan: &str, msg: &str) {
        match self.client.send_privmsg(chan, msg) {
            Ok(_) => (),
            Err(e) => print!("failed to send privmsg: {}", e)
        }
    }

    fn load_module(&mut self, name: &str) {
        match Library::new(format!("libmod_{}.so", name)) {
            Ok(lib) => {
                let m = Module{
                    name: name.to_string(),
                    lib: lib,
                };
                let meta = m.get_meta();
                for command in meta.commands.iter() {
                    self.commands.insert(command.0.to_string(), *command.1);
                }
                self.modules.insert(name.to_string(), m);
            },
            Err(e) => print!("failed to load module: {}", e)
        }
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
            let result: Result<Symbol<unsafe extern fn() -> types::Meta>, io::Error> = self.lib.get(b"get_meta");
            match result {
                Ok(func) => return func(),
                Err(err) => {
                    println!("{}: failed to find get_meta function: {}", self.name, err);
                    return types::Meta{
                        commands: HashMap::new(),
                    };
                }
            };
        }
    }
}
