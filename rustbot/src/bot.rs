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
            let ctx = &mut Context {
                bot: self,
                channel: channel,
            };
            if let Some(c) = message.get(0..1) {
                if ctx.bot.conf.cmdchars.contains(c) {
                    // it's a command!
                    let parts: Vec<&str> = message[1..].splitn(2, ' ').collect();
                    if let Some(f) = ctx.bot.commands.get(parts[0]).cloned() {
                        f(ctx, parts.get(1).unwrap_or(&""))
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
        let libpath = if cfg!(debug_assertions) {
            format!("libmod_{}.so", name)
        } else {
            format!("target/release/libmod_{}.so", name)
        };
        match Library::new(libpath) {
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

struct Context<'a> {
    bot: &'a mut Bot,
    channel: String,
}

impl<'a> types::Context for Context<'a> {
    fn reply(&self, message: &str) {
        types::Bot::send_privmsg(self.bot, self.channel.as_str(), message);
    }
    fn bot(&mut self) -> &mut types::Bot {
        return self.bot;
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
    for m in b.conf.modules.clone().iter() {
        types::Bot::load_module(b, m.as_str()); // WHY
    }
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
