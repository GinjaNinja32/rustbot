use error::*;
use rusqlite::types::{FromSqlError, ValueRef};
use rusqlite::Connection;
use serenity::model::prelude as serenity;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use types::Prefix::*;
use types::Source::*;

bitflags! {
    pub struct Perms: u64 {
        const None  = 0x00000000;
        const Admin = 0x00000001;
        const TestA = 0x00000002;
        const TestB = 0x00000004;
        const TestC = 0x00000008;
    }
}

impl std::fmt::Display for Perms {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)?;
        dbg!(self.bits);

        let diff = self.bits & !Perms::all().bits;
        dbg!(diff);
        if diff != 0 {
            dbg!(());
            write!(f, " | 0x{:x}", diff)?;
        }

        Ok(())
    }
}

impl rusqlite::types::FromSql for Perms {
    fn column_result(v: ValueRef) -> std::result::Result<Perms, FromSqlError> {
        match v {
            ValueRef::Null => Ok(Perms::None),
            ValueRef::Integer(v) => Ok(Perms { bits: v as u64 }),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub type Command = Arc<Fn(&mut Context, &str) -> Result<()> + Send + Sync>;

pub struct Meta {
    commands: BTreeMap<String, Command>,
}

impl Meta {
    pub fn new() -> Meta {
        Meta {
            commands: BTreeMap::new(),
        }
    }
    pub fn command(&mut self, name: &str, f: fn(&mut Context, &str) -> Result<()>) {
        self.commands.insert(name.to_string(), Arc::new(f));
    }
    pub fn commandrc(&mut self, name: &str, f: Command) {
        self.commands.insert(name.to_string(), f);
    }
    pub fn commands(&self) -> &BTreeMap<String, Command> {
        &self.commands
    }
}

pub trait Bot {
    fn load_module(&mut self, &str) -> Result<()>;
    fn drop_module(&mut self, &str) -> Result<()>;
    fn perms(&self, Source) -> Result<Perms>;
    fn has_perm(&self, Source, Perms) -> Result<bool>;
    fn sql(&mut self) -> &Mutex<Connection>;
    fn commands(&self) -> &BTreeMap<String, Command>;

    fn irc_send_privmsg(&self, &str, &str, &str) -> Result<()>;
    fn irc_send_raw(&self, &str, &str) -> Result<()>;

    // Internal
    //fn handle(&mut self, ctx: &mut Context, msg: &str);
}

pub struct Context<'a> {
    pub bot: &'a mut Bot,
    pub source: Source,
    pub bot_name: String,
}

impl<'a> Context<'a> {
    pub fn reply(&self, message: &str) -> Result<()> {
        match &self.source {
            IRC {
                config,
                prefix,
                channel,
            } => {
                if let Some(ch) = channel {
                    if let Some(User { nick, .. }) = prefix {
                        if *ch == self.bot_name {
                            self.bot
                                .irc_send_privmsg(config.as_str(), nick.as_str(), message)?;
                        } else {
                            self.bot.irc_send_privmsg(
                                config.as_str(),
                                ch.as_str(),
                                &format!("{}: {}", nick.as_str(), message),
                            )?;
                        }
                    }
                }
            }
            Discord { channel, .. } => {
                channel.say(message).map(|_| ())?;
            }
        }

        Ok(())
    }
    pub fn has_perm(&self, flag: Perms) -> Result<bool> {
        Ok(self.perms()?.contains(flag))
    }
    pub fn perms(&self) -> Result<Perms> {
        self.bot.perms(self.source.clone())
    }

    pub fn irc_send_privmsg(&self, chan: &str, msg: &str) -> Result<()> {
        if let IRC { ref config, .. } = self.source {
            self.bot.irc_send_privmsg(config.as_str(), chan, msg)
        } else {
            Err(Error::new("ctx.irc_send_privmsg on non-IRC context"))
        }
    }

    pub fn irc_send_raw(&mut self, msg: &str) -> Result<()> {
        if let IRC { ref config, .. } = self.source {
            self.bot.irc_send_raw(config.as_str(), msg)
        } else {
            Err(Error::new("ctx.irc_send_privmsg on non-IRC context"))
        }
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    IRC {
        config: String,
        prefix: Option<Prefix>,
        channel: Option<String>,
    },
    Discord {
        user: serenity::User,
        channel: serenity::ChannelId,
        guild: Option<serenity::GuildId>,
    },
}

#[derive(Debug, Clone)]
pub enum Prefix {
    Server(String),
    User {
        nick: String,
        user: String,
        host: String,
    },
}

impl std::fmt::Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Server(s) => write!(f, "{}", s),
            User { nick, user, host } => write!(f, "{}!{}@{}", nick, user, host),
        }
    }
}
