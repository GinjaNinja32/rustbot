use crate::bot;
use crate::message;
use rustbot::prelude::*;
use rustbot::types;
use serenity::model::prelude as ser;
use std::sync::Arc;

pub use self::Prefix::*;
pub use self::Source::*;

pub struct Context<'a> {
    pub bot: &'a bot::Rustbot,
    pub config: String,
    pub source: Source,
    pub bot_name: String,
}

impl<'a> types::Context for Context<'a> {
    fn bot(&self) -> &(dyn Bot + Sync) {
        self.bot
    }

    fn source_str(&self) -> String {
        format!("{}", self.source)
    }

    fn say(&self, message: &str) -> Result<()> {
        self.reply(Message::Simple(message.to_string()))
    }

    fn reply(&self, message: Message) -> Result<()> {
        match &self.source {
            IRC { prefix, channel } => {
                if let Some(ch) = channel {
                    if let Some(User { nick, .. }) = prefix {
                        if *ch == self.bot_name {
                            for msg in message::format_irc(message)? {
                                self.bot.irc_send_privmsg(&self.config, nick.as_str(), msg.as_str())?;
                            }
                        } else {
                            for msg in message::format_irc(message)? {
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
                channel.say(http, message::format_discord(message)?).map(|_| ())?;
            }
        }

        Ok(())
    }

    fn perms(&self) -> Result<Perms> {
        // TODO
        // self.bot.perms(&self.config, &self.source)
        match &self.source {
            IRC {
                prefix: Some(User { nick, user, host }),
                ..
            } => {
                let perms: Perms = match self.bot.sql().lock().query(
                    "SELECT flags FROM irc_permissions WHERE config_id = $1 AND nick = $2 AND username = $3 AND host = $4",
                    &[&self.config, &nick, &user, &host],
                ) {
                    Err(e) => {
                        println!("error: {}", e);
                        Perms::None
                    }
                    Ok(v) => {
                        if v.is_empty() {
                            Perms::None
                        } else {
                            v.get(0).get(0)
                        }
                    }
                };
                Ok(perms)
            }
            Discord { user, .. } => {
                let perms: Perms = match self.bot.sql().lock().query(
                    "SELECT flags FROM dis_permissions WHERE config_id = $1 AND user_id = $2",
                    &[&self.config, &(*user.id.as_u64() as i64)],
                ) {
                    Err(e) => {
                        println!("error: {}", e);
                        Perms::None
                    }
                    Ok(v) => {
                        if v.is_empty() {
                            Perms::None
                        } else {
                            v.get(0).get(0)
                        }
                    }
                };
                Ok(perms)
            }
            _ => Ok(Perms::None),
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
        user: ser::User,
        channel: ser::ChannelId,
        guild: Option<ser::GuildId>,

        cache: serenity::cache::CacheRwLock,
        http: Arc<serenity::http::Http>,
    },
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            IRC { prefix, .. } => {
                if let Some(prefix) = prefix {
                    write!(f, "{}", prefix)
                } else {
                    write!(f, "None")
                }
            }
            Discord { user, guild, .. } => write!(f, "{:?}:{}", guild.map(|g| *g.as_u64()), user.id.as_u64()),
        }
    }
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
