use rusqlite::Connection;
use serenity::model::prelude as serenity;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

pub const PERM_ADMIN: u64 = 1;

pub type Command = Arc<Fn(&mut Context, &str) + Send + Sync>;

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
        self.commands.insert(name.to_string(), Arc::new(f));
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
    fn perms(&self) -> u64;
    fn has_perm(&self, u64) -> bool;
    fn get_source(&self) -> Option<Source>;
    fn bot(&mut self) -> &mut Bot;

    fn irc_send_privmsg(&self, &str, &str);
    fn irc_send_raw(&mut self, &str);
}

pub trait Bot {
    fn load_module(&mut self, &str) -> Result<(), String>;
    fn drop_module(&mut self, &str) -> Result<(), String>;
    fn perms(&self, Source) -> u64;
    fn has_perm(&self, Source, u64) -> bool;
    fn sql(&mut self) -> &Mutex<Connection>;

    fn irc_send_privmsg(&self, &str, &str, &str);
    fn irc_send_raw(&self, &str, &str);
}

#[derive(Debug, Clone)]
pub enum Source {
    IRCServer {
        config: String,
        host: String,
        channel: Option<String>,
    },
    IRCUser {
        config: String,
        nick: String,
        user: String,
        host: String,
        channel: Option<String>,
    },
    DiscordUser {
        user: serenity::User,
        channel: serenity::ChannelId,
        guild: Option<serenity::GuildId>,
    },
}
