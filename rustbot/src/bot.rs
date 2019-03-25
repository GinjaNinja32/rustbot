use db;
use irc::client::ext::ClientExt;
use irc::client::prelude as irc;
use irc::client::prelude::Client;
use libloading::{Library, Symbol};
use parking_lot::{Mutex, RwLock};
use rusqlite::{Connection, NO_PARAMS};
use serenity::model::channel;
use serenity::prelude as dis;
use shared::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;

struct Rustbot {
    clients: RwLock<BTreeMap<String, Arc<irc::IrcClient>>>,
    db: Mutex<rusqlite::Connection>,
    modules: RwLock<BTreeMap<String, Module>>,
    commands: RwLock<BTreeMap<String, Command>>,
}

impl Rustbot {
    fn irc_parse_prefix(&self, prefix: Option<String>) -> Option<Prefix> {
        match prefix {
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
        }
    }

    fn irc_incoming(&self, cfg: String, bot_name: &str, irc_msg: irc::Message) {
        if let irc::Command::PRIVMSG(channel, message) = irc_msg.command {
            let source = IRC {
                config: cfg.clone(),
                prefix: self.irc_parse_prefix(irc_msg.prefix),
                channel: Some(channel),
            };
            let ctx = &mut Context {
                bot: self,
                source: source,
                bot_name: bot_name.to_string(),
            };
            Rustbot::handle(self, ctx, message.as_str());
        }
    }

    fn dis_incoming(&self, msg: channel::Message) {
        let ctx = &mut Context {
            bot: self,
            source: Discord {
                user: msg.author,
                channel: msg.channel_id,
                guild: msg.guild_id,
            },
            bot_name: "".to_string(),
        };

        Rustbot::handle(self, ctx, msg.content.as_str());
    }

    fn handle(&self, ctx: &mut Context, message: &str) {
        let cmdchars: String = {
            let db = ctx.bot.sql().lock();
            match ctx.source {
                IRC { ref config, .. } => db
                    .query_row("SELECT cmdchars FROM irc_config WHERE id = ?", vec![config], |row| {
                        row.get(0)
                    })
                    .unwrap(),
                Discord { .. } => db
                    .query_row("SELECT cmdchars FROM dis_config", NO_PARAMS, |row| row.get(0))
                    .unwrap(),
            }
        };
        if let Some(c) = message.get(0..1) {
            if cmdchars.contains(c) {
                // it's a command!
                let parts: Vec<&str> = message[1..].splitn(2, ' ').collect();
                let res = self.commands.read().get(parts[0]).cloned();
                if let Some(f) = res {
                    let result = f.call(ctx, parts.get(1).unwrap_or(&""));
                    result
                        .or_else(|e| ctx.reply(&format!("command failed: {}", e)))
                        .err()
                        .map(|e| println!("failed to handle error: {}", e));
                }
                return;
            }
        }
    }
}

impl shared::types::Bot for Rustbot {
    fn drop_module(&self, name: &str) -> Result<()> {
        if let Some(m) = self.modules.write().remove(name) {
            let db = self.db.lock();
            db
                .execute(
                    "INSERT INTO modules (name, enabled) VALUES (?, false) ON CONFLICT (name) DO UPDATE SET enabled = false",
                    vec![name],
                )?;
            let meta = m.get_meta()?;
            let mut commands = self.commands.write();
            for command in meta.commands().iter() {
                commands.remove(command.0);
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn load_module(&self, name: &str) -> Result<()> {
        let libpath = if cfg!(debug_assertions) {
            format!("libmod_{}.so", name)
        } else {
            format!("target/release/libmod_{}.so", name)
        };
        let lib = Library::new(libpath)?;

        self.db.lock().execute(
            "INSERT INTO modules (name, enabled) VALUES (?, true) ON CONFLICT (name) DO UPDATE SET enabled = true",
            vec![name],
        )?;
        let m = Module { lib };
        let meta = m.get_meta()?;
        let mut commands = self.commands.write();
        for command in meta.commands().iter() {
            commands.insert(command.0.to_string(), (*command.1).clone());
        }
        self.modules.write().insert(name.to_string(), m);
        Ok(())
    }

    fn perms(&self, who: Source) -> Result<Perms> {
        match who {
            IRC {
                config: c,
                prefix:
                    Some(User {
                        nick: n,
                        user: u,
                        host: h,
                    }),
                ..
            } => {
                let perms: Perms = match self.db.lock().query_row(
                    "SELECT flags FROM irc_permissions WHERE config_id = ? AND nick = ? AND user = ? AND host = ?",
                    vec![c, n, u, h],
                    |row| row.get(0),
                ) {
                    Err(rusqlite::Error::QueryReturnedNoRows) => Perms::None,
                    Err(e) => {
                        println!("error: {}", e);
                        Perms::None
                    }
                    Ok(v) => v,
                };
                Ok(perms)
            }
            Discord { user, .. } => {
                let perms: Perms = match self.db.lock().query_row(
                    "SELECT flags FROM dis_permissions WHERE user_id = ?",
                    vec![*user.id.as_u64() as i64],
                    |row| row.get(0),
                ) {
                    Err(rusqlite::Error::QueryReturnedNoRows) => Perms::None,
                    Err(e) => {
                        println!("error: {}", e);
                        Perms::None
                    }
                    Ok(v) => v,
                };
                Ok(perms)
            }
            _ => Ok(Perms::None),
        }
    }

    fn sql(&self) -> &Mutex<Connection> {
        &self.db
    }

    fn irc_send_privmsg(&self, cfg: &str, channel: &str, message: &str) -> Result<()> {
        if let Some(client) = self.clients.read().get(cfg) {
            client.send_privmsg(channel, message)?;
            Ok(())
        } else {
            Err(Error::new("invalid configid"))
        }
    }

    fn irc_send_raw(&self, cfg: &str, line: &str) -> Result<()> {
        if let Some(client) = self.clients.read().get(cfg) {
            client.send(line)?;
            Ok(())
        } else {
            Err(Error::new("invalid configid"))
        }
    }
}

pub fn start() -> Result<()> {
    let b = Arc::new(Rustbot {
        clients: RwLock::new(BTreeMap::new()),
        db: Mutex::new(db::open().unwrap()),
        modules: RwLock::new(BTreeMap::new()),
        commands: RwLock::new(BTreeMap::new()),
    });

    let mut configs: Vec<(String, irc::Config)> = {
        let db = b.db.lock();
        let mut stmt = db.prepare("SELECT id, nick, user, real, server, port, ssl FROM irc_config")?;
        let result: std::result::Result<Vec<(String, irc::Config)>, rusqlite::Error> = stmt
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
            })?
            .collect();

        result?
    };

    for (id, conf) in configs.iter_mut() {
        let db = b.db.lock();
        let cid = id.clone();
        conf.channels = db
            .prepare("SELECT channel FROM irc_channels WHERE config_id = ?")
            .and_then(|mut stmt| stmt.query_map(vec![cid], |row| row.get(0)).and_then(|c| c.collect()))?;
    }

    {
        let modules: Vec<String> = {
            let db = b.db.lock();
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
        let client = Arc::new(irc::IrcClient::from_config(conf.clone())?);
        client.send_cap_req(&[irc::Capability::MultiPrefix])?;
        client.identify()?;

        {
            b.clients.write().insert(id.clone(), client.clone());
        }
    }

    for (id, conf) in configs {
        let b = b.clone();
        thread::Builder::new()
            .name(format!(
                "IRC: {} ({}:{})",
                id,
                conf.server.expect("a non-None server address"),
                conf.port.expect("a non-None server port")
            ))
            .spawn(move || {
                let client = { b.clients.read().get(&id).unwrap().clone() };
                client
                    .for_each_incoming(|irc_msg| {
                        let b = b.clone();
                        let id = id.clone();
                        rayon::spawn(move || {
                            let client = { b.clients.read().get(&id).unwrap().clone() };
                            b.irc_incoming(id.clone(), client.current_nickname(), irc_msg);
                        });
                    })
                    .unwrap();
            })?;
    }

    let token: String = {
        b.db.lock()
            .query_row("SELECT bot_token FROM dis_config", NO_PARAMS, |row| row.get(0))
            .unwrap()
    };
    let mut dis = dis::Client::new(&token, DiscordBot { bot: b }).unwrap();
    dis.start()?;
    Ok(())
}

struct DiscordBot {
    bot: Arc<Rustbot>,
}

impl dis::EventHandler for DiscordBot {
    fn message(&self, _disctx: dis::Context, msg: channel::Message) {
        let bot = self.bot.clone();
        rayon::spawn(move || {
            bot.dis_incoming(msg);
        });
    }
}

struct Module {
    //name: String,
    lib: Library,
}

impl Module {
    fn get_meta(&self) -> Result<Meta> {
        unsafe {
            let sym: Symbol<Option<unsafe fn() -> Meta>> = self.lib.get(b"get_meta")?;
            match Symbol::lift_option(sym) {
                Some(f) => Ok(f()),
                None => Err(Error::new("symbol not found")),
            }
        }
    }
}
