#![allow(non_upper_case_globals)]

use parking_lot::Mutex;
use rusqlite::types::{FromSqlError, ValueRef};
use rusqlite::Connection;
use serenity::model::prelude as serenity;
use std::collections::BTreeMap;
use std::sync::Arc;

use error::*;
use types::Message::*;
use types::Prefix::*;
use types::Source::*;

bitflags! {
    pub struct Perms: u64 {
        const None     = 0x00000000;
        const Admin    = 0x00000001;
        const Raw      = 0x00000002;
        const Database = 0x00000004;
        const Eval     = 0x00000008;
        const Modules  = 0x00000010;
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

pub type CommandFn = Fn(&Context, &str) -> Result<()> + Send + Sync;
#[derive(Clone)]
pub struct Command {
    pub function: Arc<CommandFn>,
    pub req_perms: Perms,
}

impl Command {
    pub fn new(f: fn(&Context, &str) -> Result<()>) -> Self {
        Self::arc(Arc::new(f))
    }
    pub fn arc(f: Arc<CommandFn>) -> Self {
        return Self {
            function: f,
            req_perms: Perms::None,
        };
    }
    pub fn req_perms(&self, p: Perms) -> Self {
        let mut s = self.clone();
        s.req_perms.insert(p);
        s
    }
    pub fn call(&self, ctx: &Context, args: &str) -> Result<()> {
        if !ctx.perms()?.contains(self.req_perms) {
            return ctx.say("permission denied");
        }

        (self.function)(ctx, args)
    }
}

pub type DeinitFn = FnMut(&Bot) -> Result<()> + Send + Sync;

pub struct Meta {
    pub(crate) commands: BTreeMap<String, Command>,
    pub(crate) deinit: Option<Box<DeinitFn>>,
}

impl Meta {
    pub fn new() -> Meta {
        Meta {
            commands: BTreeMap::new(),
            deinit: None,
        }
    }
    pub fn cmd(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_string(), cmd);
    }
    pub fn deinit(&mut self, f: Box<DeinitFn>) {
        self.deinit = Some(f)
    }
}

pub trait Bot {
    fn load_module(&self, &str) -> Result<()>;
    fn drop_module(&self, &str) -> Result<()>;
    fn perms(&self, Source) -> Result<Perms>;
    fn sql(&self) -> &Mutex<Connection>;

    fn irc_send_privmsg(&self, &str, &str, &str) -> Result<()>;
    fn irc_send_raw(&self, &str, &str) -> Result<()>;

    fn dis_send_message(&self, &str, &str, &str, bool) -> Result<()>;
}

pub struct Context<'a> {
    pub bot: &'a (Bot + Sync),
    pub source: Source,
    pub bot_name: String,
}

pub enum Message {
    Simple(String),
    Code(String),
}

fn paste_max_lines(input: String, max_lines: usize) -> Result<(Vec<String>, Option<String>)> {
    let lines: Vec<String> = input.split("\n").map(|l| l.to_string()).collect();
    if lines.len() > max_lines {
        let client = reqwest::Client::new();
        let mut result = client.post("http://ix.io").form(&[("f:1", input)]).send()?;

        let url = result.text()?;

        Ok((
            lines[0..max_lines - 1].to_vec(),
            Some(format!("[full message: {}]", url.trim())),
        ))
    } else {
        Ok((lines, None))
    }
}

impl Message {
    fn format_irc(self) -> Result<Vec<String>> {
        match self {
            Simple(s) | Code(s) => match paste_max_lines(s, 3)? {
                (lines, None) => Ok(lines),
                (mut lines, Some(extra)) => {
                    lines.push(extra);
                    Ok(lines)
                }
            },
        }
    }
    fn format_discord(self) -> Result<String> {
        match self {
            Simple(s) => match paste_max_lines(s, 11)? {
                (lines, None) => Ok(lines.join("\n")),
                (lines, Some(extra)) => Ok(format!("{}\n{}", lines.join("\n"), extra)),
            },
            Code(s) => {
                if s.contains('\n') {
                    Ok(format!("`{}`", s))
                } else {
                    match paste_max_lines(s, 11)? {
                        (lines, None) => Ok(format!("```\n{}\n```", lines.join("\n"))),
                        (lines, Some(extra)) => Ok(format!("```\n{}\n```{}", lines.join("\n"), extra)),
                    }
                }
            }
        }
    }
}

impl<'a> Context<'a> {
    pub fn say(&self, message: &str) -> Result<()> {
        self.reply(Message::Simple(message.to_string()))
    }
    pub fn reply(&self, message: Message) -> Result<()> {
        match &self.source {
            IRC {
                config,
                prefix,
                channel,
            } => {
                if let Some(ch) = channel {
                    if let Some(User { nick, .. }) = prefix {
                        if *ch == self.bot_name {
                            for msg in message.format_irc()? {
                                self.bot
                                    .irc_send_privmsg(config.as_str(), nick.as_str(), msg.as_str())?;
                            }
                        } else {
                            for msg in message.format_irc()? {
                                self.bot.irc_send_privmsg(
                                    config.as_str(),
                                    ch.as_str(),
                                    &format!("{}: {}", nick.as_str(), msg.as_str()),
                                )?;
                            }
                        }
                    }
                }
            }
            Discord { channel, .. } => {
                channel.say(message.format_discord()?).map(|_| ())?;
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

    pub fn irc_send_raw(&self, msg: &str) -> Result<()> {
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
    User { nick: String, user: String, host: String },
}

impl std::fmt::Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Server(s) => write!(f, "{}", s),
            User { nick, user, host } => write!(f, "{}!{}@{}", nick, user, host),
        }
    }
}
