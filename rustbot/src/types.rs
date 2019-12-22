#![allow(non_upper_case_globals)]

use parking_lot::Mutex;
use postgres::types::FromSql;
use postgres::Connection;
use serenity::model::prelude as serenitym;
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

impl FromSql for Perms {
    fn from_sql(
        ty: &postgres::types::Type,
        raw: &[u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        i64::from_sql(ty, raw).map(|i| Perms { bits: i as u64 })
    }
    fn accepts(ty: &postgres::types::Type) -> bool {
        i64::accepts(ty)
    }
}

pub type CommandFn = dyn Fn(&Context, &str) -> Result<()> + Send + Sync;
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
            return Ok(());
        }

        (self.function)(ctx, args)
    }
}

pub type DeinitFn = dyn FnMut(&dyn Bot) -> Result<()> + Send + Sync;

pub trait Meta {
    fn cmd(&mut self, name: &str, cmd: Command);
    fn deinit(&mut self, f: Box<DeinitFn>);
}

pub trait Bot {
    fn load_module(&self, &str) -> Result<()>;
    fn drop_module(&self, &str) -> Result<()>;
    fn perms(&self, &str, &Source) -> Result<Perms>;
    fn sql(&self) -> &Mutex<Connection>;

    fn irc_send_privmsg(&self, &str, &str, &str) -> Result<()>;
    fn irc_send_raw(&self, &str, &str) -> Result<()>;

    fn dis_send_message(&self, &str, &str, &str, &str, bool) -> Result<()>;
}

pub struct Context<'a> {
    pub bot: &'a (dyn Bot + Sync),
    pub config: String,
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
                if !s.contains('\n') {
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
            IRC { prefix, channel } => {
                if let Some(ch) = channel {
                    if let Some(User { nick, .. }) = prefix {
                        if *ch == self.bot_name {
                            for msg in message.format_irc()? {
                                self.bot.irc_send_privmsg(&self.config, nick.as_str(), msg.as_str())?;
                            }
                        } else {
                            for msg in message.format_irc()? {
                                self.bot.irc_send_privmsg(
                                    &self.config,
                                    ch.as_str(),
                                    &format!("{}: {}", nick.as_str(), msg.as_str()),
                                )?;
                            }
                        }
                    }
                }
            }
            Discord { channel, http, .. } => {
                channel.say(http, message.format_discord()?).map(|_| ())?;
            }
        }

        Ok(())
    }
    pub fn perms(&self) -> Result<Perms> {
        self.bot.perms(&self.config, &self.source)
    }

    pub fn irc_send_privmsg(&self, chan: &str, msg: &str) -> Result<()> {
        if let IRC { .. } = self.source {
            self.bot.irc_send_privmsg(&self.config, chan, msg)
        } else {
            Err(Error::new("ctx.irc_send_privmsg on non-IRC context"))
        }
    }

    pub fn irc_send_raw(&self, msg: &str) -> Result<()> {
        if let IRC { .. } = self.source {
            self.bot.irc_send_raw(&self.config, msg)
        } else {
            Err(Error::new("ctx.irc_send_privmsg on non-IRC context"))
        }
    }
}

#[derive(Clone)]
pub enum Source {
    IRC {
        prefix: Option<Prefix>,
        channel: Option<String>,
    },
    Discord {
        user: serenitym::User,
        channel: serenitym::ChannelId,
        guild: Option<serenitym::GuildId>,

        cache: serenity::cache::CacheRwLock,
        http: Arc<serenity::http::Http>,
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
