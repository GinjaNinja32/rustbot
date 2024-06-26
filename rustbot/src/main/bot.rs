use ::irc::client::ext::ClientExt;
use ::irc::client::prelude as irc;
use ::irc::client::prelude::Client;
use flexi_logger::{LogSpecBuilder, Logger, LoggerHandle};
use futures::channel::oneshot::{self, Receiver, Sender};
use libloading::Library;
use log::{error, info, Level};
use parking_lot::{Mutex, RwLock};
use postgres::types::{FromSql, Type};
use regex::Regex;
use serde::Deserialize;
use serenity::model::channel;
use serenity::model::guild;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude as dis;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::str;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::config;
use super::context;
use super::context::{Prefix, Source};
use super::core;
use super::db;
use super::message;
use rustbot::prelude::{Source as LibSource, *};
use rustbot::types;

pub struct Rustbot {
    clients: RwLock<BTreeMap<String, Arc<irc::IrcClient>>>,
    caches: RwLock<BTreeMap<String, Arc<serenity::CacheAndHttp>>>,
    db: Mutex<postgres::Client>,
    modules: RwLock<BTreeMap<String, Module>>,
    core_commands: RwLock<BTreeMap<String, (Perms, Box<core::CoreCommand>)>>,
    commands: RwLock<BTreeMap<String, (String, Command)>>,
    logger: Mutex<LogInfo>,

    pub(crate) suppress_errors: RwLock<BTreeMap<String, Instant>>,
}

struct LogInfo {
    logger: LoggerHandle,
    current_level: Level,
}

fn irc_parse_prefix(prefix: Option<String>) -> Option<context::Prefix> {
    match prefix {
        None => None,
        Some(s) => {
            if !s.contains('!') {
                Some(Prefix::Server(s))
            } else {
                let ss = s.clone();
                let nr: Vec<&str> = ss.splitn(2, '!').collect();
                if !nr[1].contains('@') {
                    Some(Prefix::Server(s))
                } else {
                    let uh: Vec<&str> = nr[1].splitn(2, '@').collect();
                    Some(Prefix::User {
                        nick: nr[0].to_string(),
                        user: uh[0].to_string(),
                        host: uh[1].to_string(),
                    })
                }
            }
        }
    }
}

impl Rustbot {
    fn irc_incoming(&self, cfg: String, bot_name: &str, irc_msg: irc::Message) {
        if let irc::Command::PRIVMSG(channel, message) = irc_msg.command {
            let mut typ = HandleType::PlainMsg;

            if channel == bot_name {
                typ |= HandleType::Private;
            } else {
                typ |= HandleType::Public;
            }

            let source = Source::Irc {
                prefix: irc_parse_prefix(irc_msg.prefix),
                channel: if channel == bot_name { None } else { Some(channel) },
            };
            let ctx = &context::Context {
                bot: self,
                config: cfg,
                source,
                bot_name: bot_name.to_string(),
            };
            self.handle(ctx, typ, message.as_str());
        }
    }

    fn dis_incoming(&self, cfg: String, disctx: dis::Context, msg: channel::Message) {
        if msg.author.id == disctx.cache.read().user.id {
            return;
        }

        let mut typ = HandleType::None;

        match msg.channel_id.to_channel(&disctx) {
            Err(e) => {
                warn!("failed to determine channel type for incoming message: {}", e);
                return;
            }
            Ok(c) => match c {
                channel::Channel::Private(_) => typ |= HandleType::Private,
                channel::Channel::Group(_) => typ |= HandleType::Group,
                channel::Channel::Guild(_) => typ |= HandleType::Public,
                _ => return,
            },
        }

        let ctx = &context::Context {
            bot: self,
            config: cfg,
            source: Source::Discord {
                user: msg.author,
                channel: msg.channel_id,
                guild: msg.guild_id,

                cache: disctx.cache,
                http: disctx.http,
            },
            bot_name: String::new(),
        };

        if !msg.content.is_empty() {
            self.handle(ctx, HandleType::PlainMsg | typ, msg.content.as_str());
        }
        for att in msg.attachments {
            self.handle(ctx, HandleType::Attachment | typ, &att.proxy_url);
        }
        if msg.content.is_empty() {
            for embed in msg.embeds {
                if embed.title.is_none() && embed.description.is_none() {
                    // probably just a link, skip it
                    continue;
                }

                let mut data = vec![];
                if let Some(author) = embed.author {
                    if let Some(url) = author.url {
                        data.push(format!("{} <{}>", author.name, url));
                    } else {
                        data.push(author.name);
                    }
                }
                if let Some(title) = embed.title {
                    if let Some(url) = embed.url {
                        data.push(format!("{title} <{url}>"));
                    } else {
                        data.push(title);
                    }
                }
                if let Some(description) = embed.description {
                    data.append(&mut description.split('\n').map(str::to_string).collect());
                }
                for field in embed.fields {
                    if field.inline {
                        data.push(format!("{}: {}", field.name, field.value.replace('\n', "\t")));
                    } else {
                        data.push(format!("{}:", field.name));
                        for line in field.value.split('\n') {
                            data.push(format!("\t{line}"));
                        }
                    }
                }

                if data.is_empty() {
                    continue;
                }

                let mut spans = vec![];

                if data.len() == 1 {
                    spans.push(format!("│ {}", data.remove(0)));
                } else {
                    spans.push(format!("╽ {}", data.remove(0)));
                    let lastline = data.remove(data.len() - 1);
                    for line in data {
                        spans.push(format!("┃ {line}"));
                    }
                    spans.push(format!("╿ {lastline}"));
                }

                self.handle(ctx, HandleType::Embed | typ, &spans.join("\n"));
            }
        }
    }

    fn handle(&self, ctx: &context::Context, typ: HandleType, message: &str) {
        match self.handle_inner(ctx, typ, message) {
            Ok(()) => (),
            Err(err) => self.handle_err(ctx, err),
        }
    }
    fn handle_err(&self, ctx: &context::Context, err: Error) {
        match match err.downcast::<UserError>() {
            Ok(ue) => {
                // It's a UserError, so try to inform the user
                ctx.say(&format!("command failed: {ue}"))
                    .with_context(|| format!("failed to inform user of error {ue}"))
            }
            Err(e) => {
                // It's a backend error; let the user know _something_ happened, but print the main
                // error only to the logs
                error!("{:?}", e);
                ctx.say("command failed")
                    .context("failed to inform user of command failure")
            }
        } {
            // If we failed to inform the user, log that so it's obvious what happened
            Ok(_) => {}
            Err(e) => error!("{:?}", e),
        }
    }

    pub fn handle_inner(&self, ctx: &context::Context, mut typ: HandleType, message: &str) -> Result<()> {
        let enabled = {
            let mut db = ctx.bot().sql().lock();
            let mods: Vec<String> = db.query(
                    "SELECT name FROM modules JOIN enabled_modules USING (name) WHERE config_id = $1 AND modules.enabled",
                    &[&ctx.config],
                )?
                .iter()
                .map(|row| row.get(0))
                .collect();
            mods
        };

        if typ.contains(HandleType::PlainMsg) {
            let cmdchars: Cow<'static, str> = {
                let chars = ctx.bot().sql().lock().query(
                    "SELECT cmdchars FROM cmdchars WHERE config_id = $1 AND $2 LIKE channel ORDER BY channel DESC LIMIT 1",
                    &[&ctx.config, &ctx.source.channel_string()],
                )?;
                if chars.is_empty() {
                    Cow::Borrowed("")
                } else {
                    Cow::Owned(chars.get(0).unwrap().get(0))
                }
            };

            if message.starts_with(|c| cmdchars.contains(c)) {
                // it's a command!
                let prefix = message.chars().take(1).next().unwrap();
                let parts: Vec<&str> = message[prefix.len_utf8()..].splitn(2, char::is_whitespace).collect();

                let (cmd, args) = self.resolve_alias(parts[0], parts.get(1).unwrap_or(&""))?;

                if let Some((p, f)) = self.core_commands.read().get(&cmd) {
                    if ctx.perms()?.contains(*p) {
                        f(ctx, &args).with_context(|| format!("failed to run command {cmd:?}"))?;
                    }
                } else {
                    let res = self.commands.read().get(&cmd).cloned();
                    if let Some((m, f)) = res {
                        if enabled.contains(&m) {
                            f.call(ctx, &args)
                                .with_context(|| format!("failed to run command {cmd:?}"))?;
                        }
                    }
                }

                typ |= HandleType::Command;
                typ &= !HandleType::PlainMsg;
            }
        }

        for name in enabled {
            if let Some(m) = self.modules.read().get(&name) {
                m.with_meta::<Result<_>>(|meta| {
                    for (ty, handler) in &meta.handlers {
                        if ty.contains(typ) {
                            self.maybe_ignore_err(&name, handler(ctx, typ, message), ())
                                .with_context(|| format!("failed to run handler for module {name:?}"))?;
                        }
                    }
                    Ok(())
                })?;
            }
        }

        Ok(())
    }

    fn maybe_ignore_err<T>(&self, name: &str, res: Result<T>, on_ignore: T) -> Result<T> {
        match self.suppress_errors.read().get(name) {
            None => res,
            Some(t) => {
                if *t < Instant::now() {
                    res
                } else {
                    match res {
                        Ok(t) => Ok(t),
                        Err(e) => {
                            warn!("suppressing error from {}: {}", name, e);
                            Ok(on_ignore)
                        }
                    }
                }
            }
        }
    }

    fn resolve_alias(&self, cmd: &str, args: &str) -> Result<(String, String)> {
        let (newcmd, transforms): (String, ArgumentTransforms) = {
            let mut db = self.sql().lock();
            let rows = db.query(
                "WITH RECURSIVE resolve(depth, name, transform) AS (
                    VALUES (0, $1, null)
                    UNION ALL SELECT resolve.depth + 1, aliases.target, aliases.transform
                              FROM aliases, resolve
                              WHERE aliases.name = resolve.name
                )
                VALUES (
                    (SELECT name FROM resolve ORDER BY depth DESC LIMIT 1),
                    (to_jsonb(array(SELECT transform::jsonb FROM resolve WHERE transform IS NOT NULL ORDER BY depth ASC)))
                )",
                &[&cmd],
            )?;
            if rows.is_empty() {
                bail!("failed to resolve alias: no result rows?");
            }
            let row = rows.get(0).unwrap();

            (row.get(0), row.get(1))
        };

        let mut args = args.to_string();
        for transform in transforms.iter() {
            match transform {
                ArgumentTransform::RegexReplace { find, replace, global } => {
                    let re = Regex::new(find)?;
                    let n = usize::from(!global.unwrap_or(false));

                    args = re.replacen(args.as_str(), n, replace.as_str()).into_owned();
                }
                ArgumentTransform::ByIndex(t) => {
                    let new_args = {
                        let indexed: Vec<_> = args.split(' ').collect();
                        let mut new_args = Vec::with_capacity(usize::max(5, 2 * indexed.len()));
                        for item in t.iter() {
                            match item {
                                Index::Single(0) => new_args.extend_from_slice(&indexed),
                                Index::Single(n) => new_args.push(indexed.get((n - 1) as usize).unwrap_or(&"")),
                                Index::Multi(n) => {
                                    new_args.extend_from_slice(indexed.get((-n - 1) as usize..).unwrap_or(&[]));
                                }
                                Index::Literal(s) => new_args.push(s),
                            }
                        }
                        new_args.join(" ")
                    };
                    args = new_args;
                }
            }
        }

        Ok((newcmd, args))
    }

    fn dis_get_replacements(
        guild: impl std::ops::Deref<Target = guild::Guild>,
        reverse: bool,
    ) -> Vec<(String, String)> {
        let mut replacements = vec![];
        for (id, m) in &guild.members {
            replacements.push((format!("@{}", m.user.read().name), format!("<@{id}>")));
            if reverse {
                replacements.push((format!("@{}", m.user.read().name), format!("<@!{id}>")));
            }
        }

        for (id, r) in &guild.roles {
            replacements.push((format!("@{}", r.name), format!("<@&{id}>")));
        }

        for (id, c) in &guild.channels {
            replacements.push((format!("#{}", c.read().name), format!("<#{id}>")));
        }

        for (id, e) in &guild.emojis {
            replacements.push((format!(":{}:", e.name), format!("<:{}:{}>", e.name, id)));
        }

        replacements
    }

    fn str_max_bytes(s: &str, n: usize) -> &str {
        if s.len() <= n {
            return s;
        }

        let (last_char_inside, _) = s.char_indices().take_while(|(i, _)| *i <= n).last().unwrap();
        &s[..last_char_inside]
    }

    pub fn drop_module(&self, name: &str) -> Result<()> {
        if let Some(mut m) = self.modules.write().remove(name) {
            info!("drop module: {}", name);
            let mut db = self.db.lock();
            db
                .execute(
                    "INSERT INTO modules (name, enabled) VALUES ($1, false) ON CONFLICT (name) DO UPDATE SET enabled = false",
                    &[&name],
                )?;
            m.with_meta_mut::<Result<_>>(|meta| {
                let mut commands = self.commands.write();
                for command in &meta.commands {
                    commands.remove(command.0);
                }
                for chan in meta.unload_channels.drain(..) {
                    chan.send(()).unwrap_or(()); // Err() here means the remote end was dropped before we got here
                }
                if let Some(f) = &mut meta.deinit {
                    f(self)?;
                }
                for thread in meta.threads.drain(..) {
                    thread.join().map_err(|e| Error::msg(format!("{e:?}")))?;
                }
                Ok(())
            })?;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn load_module(&self, name: &str) -> Result<()> {
        info!("load module: {}", name);
        let libpath = if cfg!(debug_assertions) {
            format!("libmod_{name}.so")
        } else {
            format!("target/release/libmod_{name}.so")
        };
        let lib = Library::new(libpath)?;

        self.db.lock().execute(
            "INSERT INTO modules (name, enabled) VALUES ($1, true) ON CONFLICT (name) DO UPDATE SET enabled = true",
            &[&name],
        )?;
        let m = load_module(name, lib)?;
        let mut commands = self.commands.write();
        m.with_meta::<Result<_>>(|meta| {
            for command in &meta.commands {
                commands.insert(command.0.to_string(), (name.to_string(), (*command.1).clone()));
            }
            Ok(())
        })?;
        self.modules.write().insert(name.to_string(), m);
        Ok(())
    }

    pub fn set_log_level(&self, level: Level) -> Result<()> {
        self.logger.lock().current_level = level;
        self.update_logger_spec()
    }

    pub fn set_module_log_level(&self, module: &str, level: Option<Level>) -> Result<()> {
        if let Some(level) = level {
            self.db.lock().execute(
                "UPDATE modules SET log_level = $1::text::log_level WHERE name = $2",
                &[&level.to_string().to_ascii_lowercase().as_str(), &module],
            )?;
        } else {
            self.db
                .lock()
                .execute("UPDATE modules SET log_level = NULL WHERE name = $1", &[&module])?;
        }
        self.update_logger_spec()
    }

    fn update_logger_spec(&self) -> Result<()> {
        let mut logger = self.logger.lock();

        let mut builder = LogSpecBuilder::new();
        builder.default(logger.current_level.to_level_filter());

        let modules: Vec<(String, String)> = self
            .db
            .lock()
            .query(
                "SELECT name, log_level::text FROM modules WHERE log_level IS NOT NULL",
                &[],
            )?
            .iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect();

        for (m, l) in &modules {
            let level = l.parse::<Level>()?;

            builder.module(&format!("mod_{m}"), level.to_level_filter());
        }

        info!("setting logger spec: {:?}", builder);

        logger.logger.set_new_spec(builder.build());

        Ok(())
    }
}

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
            serde::de::Unexpected::Unsigned(other.into()),
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

impl FromSql<'_> for ArgumentTransforms {
    fn from_sql(
        ty: &Type,
        raw: &[u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let v = serde_json::Value::from_sql(ty, raw)?;
        Ok(serde_json::from_value(v)?)
    }

    fn accepts(ty: &Type) -> bool {
        serde_json::Value::accepts(ty)
    }
}

impl types::Bot for Rustbot {
    fn sql(&self) -> &Mutex<postgres::Client> {
        &self.db
    }

    fn irc_send_privmsg(&self, cfg: &str, channel: &str, message: &str) -> Result<()> {
        if let Some(client) = self.clients.read().get(cfg) {
            let message = Self::str_max_bytes(message, 490);
            client.send_privmsg(channel, message).map_err(from_irc)?;
            Ok(())
        } else {
            bail!("invalid configid")
        }
    }

    fn irc_send_raw(&self, cfg: &str, line: &str) -> Result<()> {
        if let Some(client) = self.clients.read().get(cfg) {
            let line = Self::str_max_bytes(line, 510);
            client.send(line).map_err(from_irc)?;
            Ok(())
        } else {
            bail!("invalid configid")
        }
    }

    fn dis_unprocess_message(&self, config: &str, guild: &str, message: &str) -> Result<String> {
        let cache_and_http = match self.caches.read().get(config) {
            None => bail!("no cache found for config {:?}", config),
            Some(c) => Arc::clone(c),
        };

        let cache = cache_and_http.cache.read();

        let mut message = message.to_string();

        let guildobj = {
            if let Ok(id) = guild.parse() {
                cache.guilds.get(&GuildId(id))
            } else {
                let mut v = None;
                for g in cache.guilds.values() {
                    if g.read().name == guild {
                        v = Some(g);
                        break;
                    }
                }
                v
            }
        }
        .ok_or_else(|| Error::msg("guild not found"))?
        .read();

        let mut replacements = Self::dis_get_replacements(guildobj, true);

        replacements.sort_by(|l, r| {
            if l.1.len() != r.1.len() {
                return l.1.len().cmp(&r.1.len()).reverse();
            }

            l.1.cmp(&r.1)
        });

        for (replace, find) in replacements {
            message = message.replace(&find, &replace);
        }

        Ok(message)
    }

    fn dis_send_message(&self, config: &str, guild: &str, channel: &str, message: &str, process: bool) -> Result<()> {
        let cache_and_http = match self.caches.read().get(config) {
            None => bail!("no cache found for config {:?}", config),
            Some(c) => Arc::clone(c),
        };

        let cache = cache_and_http.cache.read();

        let guildobj = {
            if let Ok(id) = guild.parse() {
                cache.guilds.get(&GuildId(id))
            } else {
                let mut v = None;
                for g in cache.guilds.values() {
                    if g.read().name == guild {
                        v = Some(g);
                        break;
                    }
                }
                v
            }
        }
        .ok_or_else(|| Error::msg("guild not found"))?
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
        .ok_or_else(|| Error::msg("channel not found"))?;

        if process {
            let mut message = message.to_string();

            let mut replacements = Self::dis_get_replacements(guildobj, false);

            replacements.sort_by(|l, r| {
                if l.0.len() != r.0.len() {
                    return l.0.len().cmp(&r.0.len()).reverse();
                }

                l.0.cmp(&r.0)
            });

            {
                for (find, replace) in replacements {
                    let mut need_replace = false;

                    let is_replace_before_ok = |c| {
                        let cat = unic_ucd::GeneralCategory::of(c);

                        cat.is_separator() || cat.is_punctuation()
                    };

                    // Check whether we actually need to do anything.
                    // Most of the time, we don't, so we can avoid allocating.
                    if message.ends_with(&find) {
                        need_replace = true;
                    } else {
                        for part in message.split(&find).skip(1) {
                            if part.starts_with(is_replace_before_ok) {
                                need_replace = true;
                            }
                        }
                    }

                    if need_replace {
                        let mut parts = message.split(&find);
                        let mut new_parts = vec![parts.next().unwrap()];

                        for part in parts {
                            if part.is_empty() || part.starts_with(is_replace_before_ok) {
                                new_parts.push(&replace);
                            } else {
                                new_parts.push(&find);
                            }
                            new_parts.push(part);
                        }

                        message = new_parts.join("");
                    }
                }
            }

            chanid.say(Arc::clone(&cache_and_http.http), message)?;
        } else {
            chanid.say(Arc::clone(&cache_and_http.http), message)?;
        }

        Ok(())
    }

    fn send_message(&self, config: &str, source: &str, msg: Message) -> Result<()> {
        let parts: Vec<_> = source.split(':').collect();
        if parts[0] == "irc" && parts.len() == 2 {
            let msg = message::format_irc(msg)?;
            for line in msg {
                self.irc_send_privmsg(config, parts[1], &line)?;
            }
            Ok(())
        } else if parts[0] == "dis" && parts.len() == 3 {
            self.dis_send_message(config, parts[1], parts[2], &message::format_discord(msg)?, true)
        } else {
            bail!("invalid source")
        }
    }
}

const LOG_MODULE_PATH_MAX_LEN: usize = 25;
// Smart module path truncation.
// Truncated segments are indicated by `segm~`.
// Omitted segments are indicated by `..` instead of `::`
// For example, shortening `some::module::path` will yield:
//   `some..` for n=6 and n=7
//   `some..pa~` at n=9
//   `some..path` for n in 10..=13
//   `some::mo~::path` at n=15
//   `some::module::path` for n>=18
pub(crate) fn truncate_module_path(s: &str, n: usize) -> Cow<'_, str> {
    // If n is less than four, we can't even show `x~..`.
    let n = if n < 4 { 4 } else { n };

    if s.len() <= n {
        return s.into();
    }
    let parts: Vec<_> = s.split("::").collect();

    let first = parts[0];

    if first.len() + 2 > n {
        return format!("{}~..", &first[..n - 3]).into();
    }

    let mut len_so_far = 2 + first.len();
    for i in (1..parts.len()).rev() {
        let this_len = parts[i].len();

        if len_so_far + 4 + this_len <= n {
            // This segment will fit, with room to spare for at least 'x~::'
            len_so_far += 2 + this_len;
            continue;
        }

        let first_to_this = if i == 1 { "::" } else { ".." };

        if len_so_far + this_len <= n {
            // This segment will entirely fit
            return format!("{}{}{}", first, first_to_this, parts[i..].join("::")).into();
        }

        if len_so_far + 1 < n {
            // Part of this segment will fit
            let fitting_part = &parts[i][..n - len_so_far - 1];

            if i + 1 == parts.len() {
                return format!("{first}{first_to_this}{fitting_part}~").into();
            } else {
                return format!(
                    "{}{}{}~::{}",
                    first,
                    first_to_this,
                    fitting_part,
                    parts[i + 1..].join("::")
                )
                .into();
            }
        }

        return format!("{}{}{}", first, first_to_this, parts[i + 1..].join("::")).into();
    }

    // Exiting the above loop implies all segments fit unmodified into n characters, but we checked
    // whether that's the case and returned early, so we can't get here unless there's a bug in the
    // logic.
    unreachable!();
}

pub fn start() -> Result<()> {
    // Initialise logging
    let logger = Logger::with_str("info")
        .format(|w, now, record| {
            write!(
                w,
                "{} {:5} {:>mod_len$}:{}: {}",
                now.now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.module_path().map_or_else(
                    || "<unnamed>".into(),
                    |e| truncate_module_path(e, LOG_MODULE_PATH_MAX_LEN)
                ),
                record.line().unwrap_or(0),
                &record.args(),
                mod_len = LOG_MODULE_PATH_MAX_LEN,
            )
        })
        .start()?;

    // Load the config
    let config = config::load()?;

    let b = Arc::new(Rustbot {
        clients: RwLock::new(BTreeMap::new()),
        caches: RwLock::new(BTreeMap::new()),
        db: Mutex::new(db::open(&config.postgres)?),
        modules: RwLock::new(BTreeMap::new()),
        core_commands: RwLock::new(core::get_commands()),
        commands: RwLock::new(BTreeMap::new()),
        logger: Mutex::new(LogInfo {
            logger,
            current_level: Level::Info,
        }),
        suppress_errors: RwLock::new(BTreeMap::new()),
    });

    b.update_logger_spec()?;

    {
        let modules: Vec<String> = {
            let mut db = b.db.lock();
            db.query("SELECT name FROM modules WHERE enabled = true", &[])?
                .iter()
                .map(|row| row.get(0))
                .collect()
        };
        for m in modules {
            b.load_module(m.as_str()).unwrap();
        }
    }

    for c in config.irc {
        let channels: Vec<String> = {
            let mut db = b.db.lock();
            let cid = c.id.clone();
            db.query("SELECT channel FROM irc_channels WHERE config_id = $1", &[&cid])?
                .iter()
                .map(|row| row.get(0))
                .collect()
        };

        let b = b.clone();
        thread::Builder::new()
            .name(format!("IRC: {}", irc_descriptor(&c)))
            .spawn(move || {
                run_with_backoff(&format!("IRC connection for {}", c.id), &|| {
                    let client = Arc::new(
                        irc::IrcClient::from_config(irc::Config {
                            nickname: Some(c.nick.clone()),
                            username: Some(c.user.clone()),
                            realname: Some(c.real.clone()),
                            server: Some(c.server.clone()),
                            port: Some(c.port),
                            use_ssl: Some(c.ssl),
                            channels: Some(channels.clone()),
                            password: c.pass.clone(),
                            ..Default::default()
                        })
                        .map_err(from_irc)?,
                    );
                    client.send_cap_req(&[irc::Capability::MultiPrefix]).map_err(from_irc)?;
                    client.identify().map_err(from_irc)?;
                    b.clients.write().insert(c.id.clone(), client.clone());
                    info!("connect: {}", irc_descriptor(&c));
                    client
                        .for_each_incoming(|irc_msg| {
                            let b = b.clone();
                            let id = c.id.clone();
                            rayon::spawn(move || {
                                let client = { b.clients.read().get(&id).unwrap().clone() };
                                b.irc_incoming(id.clone(), client.current_nickname(), irc_msg);
                            });
                        })
                        .map_err(from_irc)?;
                    Ok(())
                });
            })?;
    }

    for c in config.discord {
        let b = b.clone();
        thread::Builder::new()
            .name(format!("Discord: {}", c.id.clone()))
            .spawn(move || {
                run_with_backoff("Discord connection", &|| {
                    let mut dis = dis::Client::new(
                        &c.token,
                        DiscordBot {
                            id: c.id.clone(),
                            bot: b.clone(),
                        },
                    )?;

                    b.caches.write().insert(c.id.clone(), dis.cache_and_http.clone());
                    info!("connect: {}", c.id);
                    dis.start()?;
                    Ok(())
                });
            })?;
    }
    Ok(())
}

fn run_with_backoff(desc: &str, f: &dyn Fn() -> Result<()>) {
    let backoff_durations: &[u64] = &[0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
    let mut b = 0; // current backoff level
    loop {
        let start = Instant::now();
        match f() {
            Ok(()) => return,
            Err(e) => error!("{} failed: {}", desc, e),
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

fn irc_descriptor(c: &config::Irc) -> String {
    format!("{} ({}:{})", c.id, c.server, c.port,)
}

struct DiscordBot {
    id: String,
    bot: Arc<Rustbot>,
}

impl dis::EventHandler for DiscordBot {
    fn message(&self, disctx: dis::Context, msg: channel::Message) {
        let id = self.id.clone();
        let bot = self.bot.clone();
        rayon::spawn(move || {
            bot.dis_incoming(id, disctx, msg);
        });
    }
}

use ouroboros::self_referencing;

#[self_referencing]
pub struct Module {
    lib: Box<libloading::Library>,
    #[borrows(lib)]
    meta: Meta,
}

fn load_module(name: &str, lib: Library) -> Result<Module> {
    let m = Module::try_new(Box::new(lib), |lib| {
        let get_meta = unsafe { lib.get::<unsafe fn(&mut dyn types::Meta)>(b"get_meta") };
        match get_meta {
            Ok(f) => {
                let mut m = Meta::new();
                unsafe {
                    f(&mut m);
                }
                Ok(m)
            }
            Err(e) => {
                let get_meta_conf =
                    unsafe { lib.get::<unsafe fn(&mut dyn types::Meta, toml::Value) -> Result<()>>(b"get_meta_conf") };
                match get_meta_conf {
                    Ok(f) => {
                        if let Some(c) = config::load()?.module.remove(name) {
                            let mut m = Meta::new();
                            unsafe {
                                f(&mut m, c)?;
                            }
                            Ok(m)
                        } else {
                            bail!("required config not passed")
                        }
                    }
                    Err(e2) => bail!("{}, {}", e, e2),
                }
            }
        }
    })?;

    Ok(m)
}

pub struct Meta {
    commands: BTreeMap<String, Command>,
    deinit: Option<Box<DeinitFn>>,
    handlers: Vec<(HandleType, Box<MsgHandlerFn>)>,
    unload_channels: Vec<Sender<()>>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Meta {
    fn new() -> Self {
        Self {
            commands: BTreeMap::new(),
            deinit: None,
            handlers: Vec::new(),
            unload_channels: Vec::new(),
            threads: Vec::new(),
        }
    }
}

impl types::Meta for Meta {
    fn cmd(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_string(), cmd);
    }
    fn deinit(&mut self, f: Box<DeinitFn>) {
        self.deinit = Some(f);
    }
    fn handle(&mut self, typ: HandleType, f: Box<MsgHandlerFn>) {
        self.handlers.push((typ, f));
    }
    fn on_unload_channel(&mut self) -> Receiver<()> {
        let (send, recv) = oneshot::channel();

        self.unload_channels.push(send);
        recv
    }
    fn thread(&mut self, f: Box<ThreadFn>) {
        self.threads.push(std::thread::spawn(f));
    }
}

fn from_irc(e: ::irc::error::IrcError) -> Error {
    Error::msg(format!("{e}"))
}
