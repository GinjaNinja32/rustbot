use crate::bot;
use crate::message;
use rustbot::prelude::*;
use rustbot::types;
use serenity::model::prelude as ser;
use std::borrow::Cow;
use std::sync::Arc;

pub struct Context<'a> {
    pub bot: &'a bot::Rustbot,
    pub config: String,
    pub source: Source,
    pub bot_name: String,
}

impl<'a> Context<'a> {
    fn reply_impl(&self, source: &Source, message: Message) -> Result<()> {
        match source {
            Source::Irc { prefix, channel } => {
                if let Some(Prefix::User { nick, .. }) = prefix {
                    match channel {
                        None => {
                            for msg in message::format_irc(message)? {
                                self.bot.irc_send_privmsg(&self.config, nick.as_str(), msg.as_str())?;
                            }
                        }

                        Some(ch) => {
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
            Source::Discord { channel, http, .. } => {
                channel.say(http, message::format_discord(message)?).map(|_| ())?;
            }
            Source::Sub { parent, .. } => return self.reply_impl(parent, message),
        }

        Ok(())
    }
}

impl<'a> types::Context for Context<'a> {
    fn config_id(&self) -> &str {
        &self.config
    }

    fn bot(&self) -> &(dyn Bot + Sync) {
        self.bot
    }

    fn source(&self) -> &dyn types::Source {
        &self.source
    }

    fn say(&self, message: &str) -> Result<()> {
        self.reply(Message::Simple(message.to_string()))
    }

    fn reply(&self, message: Message) -> Result<()> {
        self.reply_impl(&self.source, message)
    }

    fn perms(&self) -> Result<Perms> {
        // TODO
        // self.bot.perms(&self.config, &self.source)
        match &self.source {
            Source::Irc {
                prefix: Some(Prefix::User { nick, user, host }),
                ..
            } => {
                let perms: Perms = match self.bot.sql().lock().query(
                    "SELECT flags FROM irc_permissions WHERE config_id = $1 AND nick = $2 AND username = $3 AND host = $4",
                    &[&self.config, &nick, &user, &host],
                ) {
                    Err(e) => {
                        error!("error fetching perms: {}", e);
                        Perms::None
                    }
                    Ok(v) => {
                        if v.is_empty() {
                            Perms::None
                        } else {
                            v.get(0).unwrap().get(0)
                        }
                    }
                };
                Ok(perms)
            }
            Source::Irc { .. } => Ok(Perms::None),
            Source::Discord { user, .. } => {
                let perms: Perms = match self.bot.sql().lock().query(
                    "SELECT flags FROM dis_permissions WHERE config_id = $1 AND user_id = $2",
                    &[&self.config, &(*user.id.as_u64() as i64)],
                ) {
                    Err(e) => {
                        error!("error fetching perms: {}", e);
                        Perms::None
                    }
                    Ok(v) => {
                        if v.is_empty() {
                            Perms::None
                        } else {
                            v.get(0).unwrap().get(0)
                        }
                    }
                };
                Ok(perms)
            }
            Source::Sub { .. } => Ok(Perms::None), // TODO
        }
    }

    fn do_sub(&self, name: &str, msg: &str) -> Result<()> {
        self.bot.handle_inner(
            &Context {
                bot: self.bot,
                config: self.config.clone(),
                source: Source::Sub {
                    parent: Box::new(self.source.clone()),
                    name: name.to_string(),
                },
                bot_name: self.bot_name.clone(),
            },
            HandleType::PlainMsg,
            msg,
        )
    }
}

#[derive(Clone)]
pub enum Source {
    Irc {
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
    Sub {
        parent: Box<Source>,
        name: String,
    },
}

impl types::Source for Source {
    fn user_string(&self) -> Cow<str> {
        match self {
            Source::Irc { prefix, .. } => {
                if let Some(prefix) = prefix {
                    format!("{prefix}").into()
                } else {
                    "none".into()
                }
            }
            Source::Discord { user, guild, .. } => {
                format!("{:?}:{}", guild.map(|g| *g.as_u64()), user.id.as_u64()).into()
            }
            Source::Sub { parent, name } => format!("{}@{}", parent.user_string(), name).into(),
        }
    }

    fn user_pretty(&self) -> Cow<str> {
        match self {
            Source::Irc { prefix, .. } => match prefix {
                Some(Prefix::User { nick, .. }) => nick.into(),
                Some(Prefix::Server(s)) => s.into(),
                None => "???".into(),
            },
            Source::Discord { user, .. } => (&user.name).into(),
            Source::Sub { name, .. } => name.into(),
        }
    }

    fn channel_string(&self) -> Cow<str> {
        match self {
            Source::Irc { channel, .. } => {
                if let Some(channel) = channel {
                    format!("irc:{channel}")
                } else {
                    "irc:query".to_string()
                }
            }
            Source::Discord { channel, guild, .. } => format!(
                "dis:{}:{}",
                guild
                    .map(|g| format!("{}", *g.as_u64()))
                    .unwrap_or_else(|| "none".to_string()),
                channel.as_u64()
            ),
            Source::Sub { parent, .. } => parent.channel_string().into_owned(),
        }
        .into()
    }

    fn get_discord_params(&self) -> Option<(Option<u64>, u64, u64)> {
        if let Source::Discord {
            guild, channel, user, ..
        } = self
        {
            Some((guild.map(|g| *g.as_u64()), *channel.as_u64(), *user.id.as_u64()))
        } else {
            None
        }
    }

    fn get_irc_params(&self) -> Option<(Option<String>, String)> {
        if let Source::Irc { prefix, channel, .. } = self {
            match prefix {
                Some(Prefix::User { nick, .. }) => Some((channel.clone(), nick.clone())),
                Some(Prefix::Server(s)) => Some((channel.clone(), s.clone())),
                None => None,
            }
        } else {
            None
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
            Self::Server(s) => write!(f, "{s}"),
            Self::User { nick, user, host } => write!(f, "{nick}!{user}@{host}"),
        }
    }
}
