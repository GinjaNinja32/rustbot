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

        let diff = self.bits & !Perms::all().bits;
        if diff != 0 {
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

pub type CommandFn = Fn(&mut Context, &str) -> Result<()> + Send + Sync;
#[derive(Clone)]
pub struct Command {
    pub function: Arc<CommandFn>,
    pub req_perms: Perms,
}

impl Command {
    pub fn new(f: fn(&mut Context, &str) -> Result<()>) -> Self {
        Self::arc(Arc::new(f))
    }
    pub fn arc(f: Arc<CommandFn>) -> Self {
        return Self {
            function: f,
            req_perms: Perms::None,
        };
    }
    pub fn req_perms(&mut self, p: Perms) -> Self {
        let mut s = self.clone();
        s.req_perms.insert(p);
        s
    }
    pub fn call(&self, ctx: &mut Context, args: &str) -> Result<()> {
        if !ctx.perms()?.contains(self.req_perms) {
            return ctx.reply("permission denied");
        }

        (self.function)(ctx, args)
    }
}

pub struct Meta {
    commands: BTreeMap<String, Command>,
}

impl Meta {
    pub fn new() -> Meta {
        Meta {
            commands: BTreeMap::new(),
        }
    }
    pub fn cmd(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_string(), cmd);
    }
    pub fn commands(&self) -> &BTreeMap<String, Command> {
        &self.commands
    }
}

pub trait Bot {
    fn load_module(&mut self, &str) -> Result<()>;
    fn drop_module(&mut self, &str) -> Result<()>;
    fn perms(&self, Source) -> Result<Perms>;
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
