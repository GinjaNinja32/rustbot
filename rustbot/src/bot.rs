use db;
use irc::client::ext::ClientExt;
use irc::client::prelude as irc;
use irc::client::prelude::Client;
use libloading::{Library, Symbol};
use parking_lot::{Mutex, RwLock};
use regex::Regex;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{Connection, NO_PARAMS};
use serde::Deserialize;
use serde_json;
use serenity::model::channel;
use serenity::model::id::*;
use serenity::prelude as dis;
use serenity::CACHE;
use rustbot::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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
            let ctx = &Context {
                bot: self,
                source: source,
                bot_name: bot_name.to_string(),
            };
            Rustbot::handle(self, ctx, message.as_str());
        }
    }

    fn dis_incoming(&self, msg: channel::Message) {
        let ctx = &Context {
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

    fn handle(&self, ctx: &Context, message: &str) {
        match self.handle_inner(ctx, message) {
            Ok(()) => (),
            Err(err) => {
                ctx.say(&format!("command failed: {}", err))
                    .err()
                    .map(|e| println!("failed to handle error: {}", e));
            }
        }
    }

    fn handle_inner(&self, ctx: &Context, message: &str) -> Result<()> {
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
        if message.starts_with(|c| cmdchars.contains(c)) {
            // it's a command!
            let prefix = message.chars().take(1).next().unwrap();
            let parts: Vec<&str> = message[prefix.len_utf8()..].splitn(2, char::is_whitespace).collect();

            let (cmd, args) = self.resolve_alias(parts[0], parts.get(1).unwrap_or(&""))?;

            let res = self.commands.read().get(&cmd).cloned();
            if let Some(f) = res {
                return f.call(ctx, &args);
            }
            return Ok(());
        }

        Ok(())
    }

    fn resolve_alias(&self, cmd: &str, args: &str) -> Result<(String, String)> {
        let (newcmd, transforms): (String, ArgumentTransforms) = {
            let db = self.sql().lock();
            db.query_row(
                "WITH resolve(depth, name, transform) AS (
                    SELECT 0, ?, null
                    UNION ALL SELECT resolve.depth + 1, aliases.target, aliases.transform
                              FROM aliases, resolve
                              WHERE aliases.name = resolve.name
                )
                SELECT max(depth), name, json_remove(json_group_array(json(transform)), '$[0]')
                FROM resolve",
                vec![cmd],
                |row| (row.get(1), row.get(2)),
            )?
        };

        let mut args = args.to_string();
        for transform in transforms.iter() {
            match transform {
                RegexReplace { find, replace, global } => {
                    let re = Regex::new(find)?;
                    args = re
                        .replacen(
                            args.as_str(),
                            if global.unwrap_or(false) { 0 } else { 1 },
                            replace.as_str(),
                        )
                        .into_owned();
                }
                ByIndex(t) => {
                    let new_args = {
                        let indexed: Vec<_> = args.split(' ').collect();
                        let mut new_args = Vec::with_capacity(usize::max(5, 2 * indexed.len()));
                        for item in t.iter() {
                            match item {
                                Index::Single(0) => new_args.extend_from_slice(&indexed),
                                Index::Single(n) => new_args.push(indexed.get((n - 1) as usize).unwrap_or(&"")),
                                Index::Multi(n) => {
                                    new_args.extend_from_slice(indexed.get((-n - 1) as usize..).unwrap_or(&[]))
                                }
                                Index::Literal(s) => new_args.push(s),
                            }
                        }
                        new_args.join(" ")
                    };
                    args = new_args
                }
            }
        }

        Ok((newcmd, args.to_string()))
    }
}

use self::ArgumentTransform::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum ArgumentTransform {
    RegexReplace {
        find: String,
        replace: String,
        #[serde(deserialize_with = "opt_bool_from_int")]
        global: Option<bool>,
    },
    ByIndex(Vec<Index>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Index {
    Single(u64),
    Multi(i64),
    Literal(String),
}

fn opt_bool_from_int<'de, D>(deserializer: D) -> std::result::Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(Some(false)),
        1 => Ok(Some(true)),
        other => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(other as u64),
            &"zero or one",
        )),
    }
}

#[derive(Deserialize, Debug)]
struct ArgumentTransforms(Vec<ArgumentTransform>);

impl std::ops::Deref for ArgumentTransforms {
    type Target = Vec<ArgumentTransform>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromSql for ArgumentTransforms {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value.as_str().and_then(|s| {
            match serde_json::from_str(s) as std::result::Result<Vec<Option<ArgumentTransform>>, serde_json::Error> {
                Ok(v) => Ok(Self(v.iter().cloned().filter_map(|v| v).collect())),
                Err(e) => Err(FromSqlError::Other(Box::new(e))),
            }
        })
    }
}

impl rustbot::types::Bot for Rustbot {
    fn drop_module(&self, name: &str) -> Result<()> {
        if let Some(m) = self.modules.write().remove(name) {
            println!("drop module: {}", name);
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
        println!("load module: {}", name);
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
                config,
                prefix: Some(User { nick, user, host }),
                ..
            } => {
                let perms: Perms = match self.db.lock().query_row(
                    "SELECT flags FROM irc_permissions WHERE config_id = ? AND nick = ? AND user = ? AND host = ?",
                    vec![config, nick, user, host],
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

    fn dis_send_message(&self, guild: &str, channel: &str, message: &str, process: bool) -> Result<()> {
        let cache = CACHE.read();

        let guildobj = {
            if let Ok(id) = guild.parse() {
                cache.guilds.get(&GuildId(id))
            } else {
                let mut v = None;
                for (_, g) in &cache.guilds {
                    if g.read().name == guild {
                        v = Some(g);
                        break;
                    }
                }
                v
            }
        }
        .ok_or_else(|| Error::new("guild not found"))?
        .read();

        let chanid = {
            if let Ok(id) = channel.parse() {
                if guildobj.channels.get(&ChannelId(id)).is_some() {
                    Some(ChannelId(id))
                } else {
                    None
                }
            } else {
                let mut v = None;
                for (id, c) in &guildobj.channels {
                    if c.read().name == channel {
                        v = Some(*id);
                        break;
                    }
                }
                v
            }
        }
        .ok_or_else(|| Error::new("channel not found"))?;

        if process {
            let mut message = message.to_string();

            let mut replacements = vec![];
            for (id, m) in &guildobj.members {
                replacements.push((format!("@{}", m.user.read().name), format!("<@{}>", id)));
            }

            for (id, r) in &guildobj.roles {
                replacements.push((format!("@{}", r.name), format!("<@&{}>", id)));
            }

            for (id, c) in &guildobj.channels {
                replacements.push((format!("#{}", c.read().name), format!("<#{}>", id)));
            }

            for (id, e) in &guildobj.emojis {
                replacements.push((format!(":{}:", e.name), format!("<:{}:{}>", e.name, id)));
            }

            replacements.sort_by(|l, r| {
                if l.0.len() != r.0.len() {
                    return l.0.len().cmp(&r.0.len()).reverse();
                }

                return l.0.cmp(&r.0);
            });

            for (find, replace) in replacements {
                message = message.replace(&find, &replace);
            }

            chanid.say(message)?;
        } else {
            chanid.say(message)?;
        }

        Ok(())
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

    for (id, conf) in configs {
        let b = b.clone();
        thread::Builder::new()
            .name(format!("IRC: {}", irc_descriptor(id.as_str(), &conf)))
            .spawn(move || {
                run_with_backoff(&format!("IRC connection for {}", id), &|| {
                    let client = Arc::new(irc::IrcClient::from_config(conf.clone())?);
                    client.send_cap_req(&[irc::Capability::MultiPrefix])?;
                    client.identify()?;
                    b.clients.write().insert(id.clone(), client.clone());
                    println!("connect: {}", irc_descriptor(id.as_str(), &conf));
                    client.for_each_incoming(|irc_msg| {
                        let b = b.clone();
                        let id = id.clone();
                        rayon::spawn(move || {
                            let client = { b.clients.read().get(&id).unwrap().clone() };
                            b.irc_incoming(id.clone(), client.current_nickname(), irc_msg);
                        });
                    })?;
                    Ok(())
                });
            })?;
    }

    let token: String = {
        b.db.lock()
            .query_row("SELECT bot_token FROM dis_config", NO_PARAMS, |row| row.get(0))
            .unwrap()
    };
    run_with_backoff("Discord connection", &|| {
        let mut dis = dis::Client::new(&token, DiscordBot { bot: b.clone() })?;
        println!("connect: discord");
        dis.start()?;
        Ok(())
    });
    Ok(())
}

fn run_with_backoff(desc: &str, f: &Fn() -> Result<()>) {
    let backoff_durations: &[u64] = &[0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
    let mut b = 0; // current backoff level
    loop {
        let start = Instant::now();
        match f() {
            Ok(()) => return,
            Err(e) => println!("{} failed: {}", desc, e),
        }

        if start.elapsed() > Duration::from_secs(60) {
            // if we ran for at least a minute, reset the backoff
            b = 0;
        }

        thread::sleep(Duration::from_secs(backoff_durations[b]));
        if b + 1 < backoff_durations.len() {
            // if we can escalate the backoff, do so
            b += 1;
        }
    }
}

fn irc_descriptor(id: &str, conf: &irc::Config) -> String {
    format!(
        "{} ({}:{})",
        id,
        conf.server.clone().expect("a non-None server address"),
        conf.port.expect("a non-None server port")
    )
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
