use error::*;
use rusqlite::Connection;
use serenity::model::prelude as serenity;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use types::Prefix::*;
use types::Source::*;

pub const PERM_ADMIN: u64 = 1;

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
    fn perms(&self, Source) -> Result<u64>;
    fn has_perm(&self, Source, u64) -> Result<bool>;
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
    pub fn has_perm(&self, flag: u64) -> Result<bool> {
        Ok((self.perms()? & flag) != 0)
    }
    pub fn perms(&self) -> Result<u64> {
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
