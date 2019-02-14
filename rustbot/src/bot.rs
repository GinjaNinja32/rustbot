use config;
use irc::client::prelude::*;
use libloading::{Library, Symbol};
use shared::types;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

struct Bot {
    client: Rc<IrcClient>,
    conf: config::Config,
    modules: HashMap<String, Module>,
    commands: HashMap<String, types::Command>,
}

impl Bot {
    fn incoming(&mut self, irc_msg: Message) {
        if let Command::PRIVMSG(channel, message) = irc_msg.command {
            let ctx = &Context {
                channel: channel,
                message: message,
            };
            if let Some(c) = ctx.message.get(0..1) {
                if self.conf.cmdchars.contains(c) {
                    // it's a command!
                    let parts: Vec<&str> = ctx.message[1..].splitn(2, ' ').collect();
                    if let Some(f) = self.commands.get(parts[0]).cloned() {
                        f(self, ctx, parts.get(1).unwrap_or(&""))
                    }
                }
            }
        }
    }
}

impl types::Bot for Bot {
    fn send_privmsg(&self, chan: &str, msg: &str) {
        if let Some(e) = self.client.send_privmsg(chan, msg).err() {
            print!("failed to send privmsg: {}", e)
        }
    }

    fn drop_module(&mut self, name: &str) {
        if let Some(m) = self.modules.remove(name) {
            match m.get_meta() {
                Ok(meta) => {
                    for command in meta.commands.iter() {
                        self.commands.remove(command.0);
                    }
                }
                Err(e) => print!("failed to get module metadata: {}", e),
            }
        }
    }

    fn load_module(&mut self, name: &str) {
        match Library::new(format!("libmod_{}.so", name)) {
            Ok(lib) => {
                let m = Module {
                    //name: name.to_string(),
                    lib,
                };
                match m.get_meta() {
                    Ok(meta) => {
                        for command in meta.commands.iter() {
                            self.commands.insert(command.0.to_string(), *command.1);
                        }
                    }
                    Err(e) => print!("failed to get module metadata: {}", e),
                }
                self.modules.insert(name.to_string(), m);
            }
            Err(e) => print!("failed to load module: {}", e),
        }
    }
}

struct Context {
    channel: String,
    message: String,
}

impl types::Context for Context {
    fn reply(&self, bot: &types::Bot, message: &str) {
        bot.send_privmsg(self.channel.as_str(), message);
    }
}

pub fn start() {
    let conf = config::load_config();
    println!("{:?}", conf);

    let client = Rc::new(IrcClient::new("conf/irc.toml").unwrap());
    let b = &mut Bot {
        client: Rc::clone(&client),
        conf,
        modules: HashMap::new(),
        commands: HashMap::new(),
    };
    client.send_cap_req(&[Capability::MultiPrefix]).unwrap();
    client.identify().unwrap();
    types::Bot::load_module(b, "admin"); // WHY
    client
        .for_each_incoming(|irc_msg| b.incoming(irc_msg))
        .unwrap();
}

struct Module {
    //name: String,
    lib: Library,
}

impl Module {
    fn get_meta(&self) -> Result<types::Meta, io::Error> {
        unsafe {
            self.lib
                .get(b"get_meta")
                .map(|f: Symbol<unsafe fn() -> types::Meta>| f())
        }
    }
}
