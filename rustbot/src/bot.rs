use config;
use irc::client::prelude::*;
use libloading::{Library, Symbol};
use shared::types;
use shared::types::Bot;
use shared::types::Source::*;
use std::collections::BTreeMap;
use std::rc::Rc;

struct IRCBot {
    client: Rc<IrcClient>,
    conf: config::Config,
    modules: BTreeMap<String, Module>,
    commands: BTreeMap<String, types::Command>,
}

impl IRCBot {
    fn incoming(&mut self, irc_msg: Message) {
        let source = match irc_msg.prefix {
            None => None,
            Some(s) => {
                if !s.contains('!') {
                    Some(Server(s))
                } else {
                    let ss = s.clone();
                    let nr: Vec<&str> = ss.splitn(2, '!').collect();
                    if !nr[1].contains('@') {
                        Some(Server(s))
                    } else {
                        let uh: Vec<&str> = nr[1].splitn(2, '@').collect();
                        Some(User {
                            nick: nr[0].to_string(),
                            user: uh[0].to_string(),
                            host: uh[1].to_string(),
                        })
                    }
                }
            }
        };
        if let Command::PRIVMSG(channel, message) = irc_msg.command {
            let ctx = &mut Context {
                bot: self,
                channel: channel,
                source: source,
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

impl Bot for IRCBot {
    fn send_privmsg(&self, chan: &str, msg: &str) {
        if let Some(e) = self.client.send_privmsg(chan, msg).err() {
            println!("failed to send privmsg: {}", e)
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
                Err(e) => println!("failed to get module metadata: {}", e),
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
                    Err(e) => println!("failed to get module metadata: {}", e),
                }
                self.modules.insert(name.to_string(), m);
            }
            Err(e) => println!("failed to load module: {}", e),
        }
    }

    fn has_perm(&self, who: &str, what: &str) -> bool {
        match self.conf.permissions.get(who) {
            Some(lst) => lst.contains(&what.to_string()),
            None => false,
        }
    }
}

struct Context<'a> {
    bot: &'a mut IRCBot,
    channel: String,
    source: Option<types::Source>,
}

impl<'a> types::Context for Context<'a> {
    fn reply(&self, message: &str) {
        if self.channel == self.bot.client.current_nickname() {
            if let Some(User { nick, .. }) = self.get_source() {
                self.bot.send_privmsg(nick.as_str(), message);
            }
        } else {
            if let Some(User { nick, .. }) = self.get_source() {
                self.bot.send_privmsg(
                    self.channel.as_str(),
                    &format!("{}: {}", nick.as_str(), message),
                );
            } else {
                self.bot.send_privmsg(self.channel.as_str(), message);
            }
        }
    }
    fn get_source(&self) -> Option<types::Source> {
        match self.source {
            Some(ref c) => Some(c.clone()),
            None => None,
        }
    }
    fn bot(&mut self) -> &mut Bot {
        self.bot
    }
    fn has_perm(&self, what: &str) -> bool {
        match self.source {
            Some(User{nick: ref n, ..}) => self.bot.has_perm(n.to_lowercase().as_str(), what),
            Some(Server(_)) => false,
            None => false
        }
    }
}

pub fn start() {
    let conf = config::load_config();
    println!("{:?}", conf);

    let client = Rc::new(IrcClient::new("conf/irc.toml").unwrap());
    let b = &mut IRCBot {
        client: Rc::clone(&client),
        conf,
        modules: BTreeMap::new(),
        commands: BTreeMap::new(),
    };
    client.send_cap_req(&[Capability::MultiPrefix]).unwrap();
    client.identify().unwrap();
    for m in b.conf.modules.clone().iter() {
        b.load_module(m.as_str());
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
    fn get_meta(&self) -> Result<types::Meta, String> {
        unsafe {
            self.lib
                .get(b"get_meta")
                .map_err(|e| format!("{}", e))
                .and_then(
                    |f: Symbol<Option<unsafe fn() -> types::Meta>>| match Symbol::lift_option(f) {
                        Some(f) => Ok(f()),
                        None => Err("symbol not found".to_string()),
                    },
                )
        }
    }
}
