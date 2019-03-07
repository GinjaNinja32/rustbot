use db;
use irc::client::prelude::*;
use libloading::{Library, Symbol};
use rusqlite::{Connection, NO_PARAMS};
use shared::types;
use shared::types::Bot;
use shared::types::Source::*;
use std::collections::BTreeMap;
use std::rc::Rc;

struct IRCBot {
    client: Rc<IrcClient>,
    db: rusqlite::Connection,
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
            ctx.handle(message.as_str());

            if let Some("\x02<") = message.get(0..2) {
                let parts: Vec<&str> = message[2..].splitn(2, ">\x02 ").collect();
                if parts.len() == 2 {
                    ctx.source = match &ctx.source {
                        Some(User { nick, user, host }) => Some(User {
                            nick: format!("@{}", parts[0].replace("\u{feff}", "")),
                            user: format!("{}-{}", nick, user),
                            host: host.to_string(),
                        }),
                        Some(Server(s)) => Some(User {
                            nick: format!("@{}", parts[0].replace("\u{feff}", "")),
                            user: "user".to_string(),
                            host: s.to_string(),
                        }),
                        None => Some(User {
                            nick: format!("@{}", parts[0].replace("\u{feff}", "")),
                            user: "user".to_string(),
                            host: "discord".to_string(),
                        }),
                    };
                    ctx.handle(parts[1]);
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

    fn drop_module(&mut self, name: &str) -> Result<(), String> {
        if let Some(m) = self.modules.remove(name) {
            self.db
                .execute(
                    "INSERT INTO modules (name, enabled) VALUES (?, false) ON CONFLICT (name) DO UPDATE SET enabled = false",
                    vec![name],
                )
                .map_err(|e| format!("{}", e))?;
            match m.get_meta() {
                Ok(meta) => {
                    for command in meta.commands().iter() {
                        self.commands.remove(command.0);
                    }
                    Ok(())
                }
                Err(e) => Err(format!("failed to get module metadata: {}", e)),
            }
        } else {
            Ok(())
        }
    }

    fn load_module(&mut self, name: &str) -> Result<(), String> {
        let libpath = if cfg!(debug_assertions) {
            format!("libmod_{}.so", name)
        } else {
            format!("target/release/libmod_{}.so", name)
        };
        match Library::new(libpath) {
            Ok(lib) => {
                self.db
                    .execute(
                        "INSERT INTO modules (name, enabled) VALUES (?, true) ON CONFLICT (name) DO UPDATE SET enabled = true",
                        vec![name],
                    )
                    .map_err(|e| format!("{}", e))?;
                let m = Module {
                    //name: name.to_string(),
                    lib,
                };
                match m.get_meta() {
                    Ok(meta) => {
                        for command in meta.commands().iter() {
                            self.commands
                                .insert(command.0.to_string(), (*command.1).clone());
                        }
                    }
                    Err(e) => return Err(format!("failed to get module metadata: {}", e)),
                }
                self.modules.insert(name.to_string(), m);
                Ok(())
            }
            Err(e) => Err(format!("failed to load module: {}", e)),
        }
    }

    fn has_perm(&self, who: types::Source, what: u64) -> bool {
        (self.perms(who) & what) != 0
    }

    fn perms(&self, who: types::Source) -> u64 {
        match who {
            User {
                nick: n,
                user: u,
                host: h,
            } => {
                let perms: i64 = match self.db.query_row(
                    "SELECT flags FROM permissions WHERE nick = ? AND user = ? AND host = ?",
                    vec![n, u, h],
                    |row| row.get(0),
                ) {
                    Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                    Err(e) => {
                        println!("error: {}", e);
                        0
                    }
                    Ok(v) => v,
                };
                return perms as u64;
            }
            Server(_) => 0,
        }
    }

    fn send_raw(&mut self, what: &str) {
        match self.client.send(what) {
            Ok(()) => (),
            Err(e) => println!("failed to send message: {}", e),
        }
    }

    fn sql(&mut self) -> &Connection {
        &self.db
    }
}

struct Context<'a> {
    bot: &'a mut IRCBot,
    channel: String,
    source: Option<types::Source>,
}

impl<'a> Context<'a> {
    fn handle(&mut self, message: &str) {
        let cmdchars: String = self
            .bot
            .db
            .query_row("SELECT cmdchars FROM config", NO_PARAMS, |row| row.get(0))
            .unwrap();
        if let Some(c) = message.get(0..1) {
            if cmdchars.contains(c) {
                // it's a command!
                let parts: Vec<&str> = message[1..].splitn(2, ' ').collect();
                if let Some(f) = self.bot.commands.get(parts[0]).cloned() {
                    f(self, parts.get(1).unwrap_or(&""));
                }
                return;
            }
        }
    }
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
    fn has_perm(&self, what: u64) -> bool {
        (self.perms() & what) != 0
    }
    fn perms(&self) -> u64 {
        if let Some(ref src) = self.source {
            return self.bot.perms(src.clone());
        }
        return 0;
    }
}

pub fn start() {
    let client = Rc::new(IrcClient::new("conf/irc.toml").unwrap());
    let b = &mut IRCBot {
        client: Rc::clone(&client),
        db: db::open().unwrap(),
        modules: BTreeMap::new(),
        commands: BTreeMap::new(),
    };
    client.send_cap_req(&[Capability::MultiPrefix]).unwrap();
    client.identify().unwrap();

    let modules: Vec<String> =
        b.db.prepare("SELECT name FROM modules WHERE enabled = true")
            .and_then(|mut stmt| {
                stmt.query_map(NO_PARAMS, |row| {
                    let s: String = row.get(0);
                    s.clone()
                })
                .and_then(|v| v.collect())
            })
            .unwrap();
    for m in modules {
        b.load_module(m.as_str()).unwrap();
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
