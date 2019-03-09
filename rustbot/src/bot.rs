use db;
use irc::client::prelude as irc;
use irc::client::prelude::*;
use libloading::{Library, Symbol};
use rusqlite::{Connection, NO_PARAMS};
use serenity::model::channel;
use serenity::prelude as serenity;
use shared::types;
use shared::types::Bot as TBot;
use shared::types::Source::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use self::BotClient::*;

struct Bot {
    clients: BTreeMap<String, Arc<IrcClient>>,
    db: Mutex<rusqlite::Connection>,
    modules: BTreeMap<String, Module>,
    commands: BTreeMap<String, types::Command>,
}

impl Bot {
    fn irc_get_source(
        &mut self,
        cfg: String,
        prefix: Option<String>,
        channel: Option<String>,
    ) -> Option<types::Source> {
        match prefix {
            None => None,
            Some(s) => {
                if !s.contains('!') {
                    Some(IRCServer {
                        config: cfg.clone(),
                        host: s,
                        channel: channel,
                    })
                } else {
                    let ss = s.clone();
                    let nr: Vec<&str> = ss.splitn(2, '!').collect();
                    if !nr[1].contains('@') {
                        Some(IRCServer {
                            config: cfg.clone(),
                            host: s,
                            channel: channel,
                        })
                    } else {
                        let uh: Vec<&str> = nr[1].splitn(2, '@').collect();
                        Some(IRCUser {
                            config: cfg.clone(),
                            nick: nr[0].to_string(),
                            user: uh[0].to_string(),
                            host: uh[1].to_string(),
                            channel: channel,
                        })
                    }
                }
            }
        }
    }

    fn irc_incoming(&mut self, client: &IrcClient, cfg: String, irc_msg: Message) {
        if let Command::PRIVMSG(channel, message) = irc_msg.command {
            let source = self.irc_get_source(cfg, irc_msg.prefix, Some(channel));
            let ctx = &mut Context {
                bot: self,
                client: IRCClient { client },
                source: source,
            };
            ctx.handle(message.as_str());
        }
    }
}

impl TBot for Bot {
    fn drop_module(&mut self, name: &str) -> Result<(), String> {
        if let Some(m) = self.modules.remove(name) {
            let db = self.db.lock().map_err(|e| format!("{}", e))?;
            db
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
                let db = self.db.lock().map_err(|e| format!("{}", e))?;
                db
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
            IRCUser {
                config: c,
                nick: n,
                user: u,
                host: h,
                ..
            } => {
                let db = self.db.lock().unwrap();
                let perms: i64 = match db.query_row(
                    "SELECT flags FROM irc_permissions WHERE config_id = ? AND nick = ? AND user = ? AND host = ?",
                    vec![c, n, u, h],
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
            IRCServer { .. } => 0,
            DiscordUser { user, .. } => {
                let db = self.db.lock().unwrap();
                let perms: i64 = match db.query_row(
                    "SELECT flags FROM dis_permissions WHERE user_id = ?",
                    vec![*user.id.as_u64() as i64],
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
        }
    }

    fn sql(&mut self) -> &Mutex<Connection> {
        &self.db
    }

    fn irc_send_privmsg(&self, cfg: &str, channel: &str, message: &str) {
        // TODO
    }

    fn irc_send_raw(&self, cfg: &str, line: &str) {
        // TODO
    }
}

struct Context<'a> {
    bot: &'a mut Bot,
    source: Option<types::Source>,
    client: BotClient<'a>,
}

impl<'a> Context<'a> {
    fn handle(&mut self, message: &str) {
        let cmdchars: String = {
            let db = self.bot.db.lock().unwrap();
            match self.source {
                Some(IRCServer { ref config, .. }) => db
                    .query_row(
                        "SELECT cmdchars FROM irc_config WHERE id = ?",
                        vec![config],
                        |row| row.get(0),
                    )
                    .unwrap(),
                Some(IRCUser { ref config, .. }) => db
                    .query_row(
                        "SELECT cmdchars FROM irc_config WHERE id = ?",
                        vec![config],
                        |row| row.get(0),
                    )
                    .unwrap(),
                Some(DiscordUser { .. }) => db
                    .query_row("SELECT cmdchars FROM dis_config", NO_PARAMS, |row| {
                        row.get(0)
                    })
                    .unwrap(),
                None => "".to_string(),
            }
        };
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
        match &self.client {
            IRCClient { client } => {
                if let Some(IRCUser {
                    channel: Some(channel),
                    nick,
                    ..
                }) = self.get_source()
                {
                    if channel == client.current_nickname() {
                        self.irc_send_privmsg(nick.as_str(), message);
                    } else {
                        self.irc_send_privmsg(
                            channel.as_str(),
                            &format!("{}: {}", nick.as_str(), message),
                        );
                    }
                }
            }
            DiscordClient { .. } => {
                if let Some(DiscordUser { channel, .. }) = self.get_source() {
                    match channel.say(message) {
                        Err(e) => println!("failed to send: {}", e),
                        Ok(_) => (),
                    }
                }
            }
        }
    }
    fn get_source(&self) -> Option<types::Source> {
        match self.source {
            Some(ref c) => Some(c.clone()),
            None => None,
        }
    }
    fn bot(&mut self) -> &mut TBot {
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

    fn irc_send_privmsg(&self, chan: &str, msg: &str) {
        if let IRCClient { client } = &self.client {
            if let Some(e) = client.send_privmsg(chan, msg).err() {
                println!("failed to send privmsg: {}", e)
            }
        }
    }

    fn irc_send_raw(&mut self, what: &str) {
        if let IRCClient { client } = &self.client {
            match client.send(what) {
                Ok(()) => (),
                Err(e) => println!("failed to send message: {}", e),
            }
        }
    }
}

pub fn start() -> Result<(), String> {
    let b = Arc::new(RwLock::new(Bot {
        clients: BTreeMap::new(),
        db: Mutex::new(db::open().unwrap()),
        modules: BTreeMap::new(),
        commands: BTreeMap::new(),
    }));

    let mut configs: Vec<(String, irc::Config)> = {
        let b = b.read().map_err(|e| format!("{}", e))?;
        let db = b.db.lock().map_err(|e| format!("{}", e))?;
        let mut stmt = db
            .prepare("SELECT id, nick, user, real, server, port, ssl FROM irc_config")
            .map_err(|e| format!("{}", e))?;
        let result: Result<Vec<(String, irc::Config)>, rusqlite::Error> = stmt
            .query_map(NO_PARAMS, |row| {
                (
                    row.get(0),
                    irc::Config {
                        nickname: row.get(1),
                        username: row.get(2),
                        realname: row.get(3),
                        server: row.get(4),
                        port: row.get(5),
                        use_ssl: row.get(6),
                        ..irc::Config::default()
                    },
                )
            })
            .map_err(|e| format!("{}", e))?
            .collect();

        result.map_err(|e| format!("{}", e))?
    };

    for (id, conf) in configs.iter_mut() {
        let b = b.read().map_err(|e| format!("{}", e))?;
        let db = b.db.lock().map_err(|e| format!("{}", e))?;
        let cid = id.clone();
        conf.channels = db
            .prepare("SELECT channel FROM irc_channels WHERE config_id = ?")
            .and_then(|mut stmt| {
                stmt.query_map(vec![cid], |row| row.get(0))
                    .and_then(|c| c.collect())
            })
            .map_err(|e| format!("{}", e))?;
    }

    {
        let mut b = b.write().map_err(|e| format!("{}", e))?;
        let modules: Vec<String> = {
            let db = b.db.lock().map_err(|e| format!("{}", e))?;
            let m = db
                .prepare("SELECT name FROM modules WHERE enabled = true")
                .and_then(|mut stmt| {
                    stmt.query_map(NO_PARAMS, |row| {
                        let s: String = row.get(0);
                        s.clone()
                    })
                    .and_then(|v| v.collect())
                })
                .unwrap();
            m
        };
        for m in modules {
            b.load_module(m.as_str()).unwrap();
        }
    }

    for (id, conf) in configs.iter() {
        println!("{}", id);
        let client = Arc::new(IrcClient::from_config(conf.clone()).map_err(|e| format!("{}", e))?);
        client
            .send_cap_req(&[Capability::MultiPrefix])
            .map_err(|e| format!("{}", e))?;
        client.identify().map_err(|e| format!("{}", e))?;

        {
            let mut b = b.write().map_err(|e| format!("{}", e))?;
            b.clients.insert(id.clone(), client.clone());
        }
    }

    for (id, _) in configs {
        let b = Arc::clone(&b);
        rayon::spawn(move || {
            let client = {
                let b = b.read().unwrap();
                b.clients.get(&id).unwrap().clone()
            };
            client
                .for_each_incoming(|irc_msg| match b.write().map_err(|e| format!("{}", e)) {
                    Ok(mut b) => b.irc_incoming(&client, id.clone(), irc_msg),
                    Err(e) => println!("failed to handle message: {}", e),
                })
                .map_err(|e| format!("{}", e))
                .unwrap();
        })
    }

    let token: String = {
        let b = b.read().unwrap();
        let db = b.db.lock().map_err(|e| format!("{}", e))?;
        db.query_row("SELECT bot_token FROM dis_config", NO_PARAMS, |row| {
            row.get(0)
        })
        .unwrap()
    };
    let mut dis = serenity::Client::new(&token, DiscordBot { bot: b }).unwrap();
    if let Err(e) = dis.start() {
        return Err(format!("discord failed: {}", e));
    };
    Ok(())
}

struct DiscordBot {
    bot: Arc<RwLock<Bot>>,
}

impl serenity::EventHandler for DiscordBot {
    fn message(&self, sctx: serenity::Context, msg: channel::Message) {
        let mut bot = self.bot.write().unwrap();
        let mut ctx = Context {
            bot: &mut bot,
            client: DiscordClient { client: sctx },
            source: Some(DiscordUser {
                user: msg.author,
                channel: msg.channel_id,
                guild: msg.guild_id,
            }),
        };

        ctx.handle(msg.content.as_str());
    }
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

enum BotClient<'a> {
    IRCClient { client: &'a IrcClient },
    DiscordClient { client: serenity::Context },
}
