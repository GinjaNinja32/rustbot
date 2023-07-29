#![allow(non_upper_case_globals)]

use bitflags::bitflags;
use parking_lot::Mutex;
use postgres::types::{FromSql, Type};
use std::borrow::Cow;
use std::sync::Arc;

use super::error::Result;
use super::spans::Span;

bitflags! {
    pub struct Perms: u64 {
        const None     = 0x0000_0000;
        const Admin    = 0x0000_0001;
        const Raw      = 0x0000_0002;
        const Database = 0x0000_0004;
        const Eval     = 0x0000_0008;
        const Modules  = 0x0000_0010;
    }
}

impl std::fmt::Display for Perms {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{self:?}")?;

        let diff = self.bits & !Perms::all().bits;
        if diff != 0 {
            write!(f, " | 0x{diff:x}")?;
        }

        Ok(())
    }
}

impl FromSql<'_> for Perms {
    fn from_sql(
        ty: &Type,
        raw: &[u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        i64::from_sql(ty, raw).map(|i| Perms { bits: i as u64 })
    }
    fn accepts(ty: &Type) -> bool {
        i64::accepts(ty)
    }
}

pub type CommandFn = dyn Fn(&dyn Context, &str) -> Result<()> + Send + Sync;
#[derive(Clone)]
pub struct Command {
    pub function: Arc<CommandFn>,
    pub req_perms: Perms,
}

impl Command {
    pub fn new<F: 'static + Fn(&dyn Context, &str) -> Result<()> + Send + Sync>(f: F) -> Self {
        Self {
            function: Arc::new(f),
            req_perms: Perms::None,
        }
    }
    #[must_use] pub fn req_perms(&self, p: Perms) -> Self {
        let mut s = self.clone();
        s.req_perms.insert(p);
        s
    }
    pub fn call(&self, ctx: &dyn Context, args: &str) -> Result<()> {
        if !ctx.perms()?.contains(self.req_perms) {
            return Ok(());
        }

        (self.function)(ctx, args)
    }
}

bitflags! {
    pub struct HandleType: u64 {
        const None       = 0x0000_0000;

        const Command    = 0x0000_0001;
        const PlainMsg   = 0x0000_0002;
        const Attachment = 0x0000_0004;
        const Embed      = 0x0000_0008;

        const Public     = 0x0000_0010;
        const Group      = 0x0000_0020;
        const Private    = 0x0000_0040;

        const All        = 0xFFFF_FFFF;
    }
}

pub type DeinitFn = dyn FnMut(&dyn Bot) -> Result<()> + Send + Sync;

pub type MsgHandlerFn = dyn Fn(&dyn Context, HandleType, &str) -> Result<()> + Send + Sync;

pub type ThreadFn = dyn FnOnce() + 'static + Send;

pub trait Meta {
    fn cmd(&mut self, name: &str, cmd: Command);
    fn deinit(&mut self, f: Box<DeinitFn>);

    fn handle(&mut self, typ: HandleType, f: Box<MsgHandlerFn>);

    fn on_unload_channel(&mut self) -> futures::channel::oneshot::Receiver<()>;

    fn thread(&mut self, f: Box<ThreadFn>);
}

pub trait Bot {
    fn sql(&self) -> &Mutex<postgres::Client>;

    fn irc_send_privmsg(&self, _: &str, _: &str, _: &str) -> Result<()>;
    fn irc_send_raw(&self, _: &str, _: &str) -> Result<()>;

    fn dis_unprocess_message(&self, _: &str, _: &str, _: &str) -> Result<String>;
    fn dis_send_message(&self, _: &str, _: &str, _: &str, _: &str, _: bool) -> Result<()>;

    fn send_message(&self, _: &str, _: &str, _: Message) -> Result<()>;
}

pub trait Context {
    fn config_id(&self) -> &str;
    fn bot(&self) -> &(dyn Bot + Sync);
    fn say(&self, _: &str) -> Result<()>;
    fn reply(&self, _: Message) -> Result<()>;
    fn perms(&self) -> Result<Perms>;
    fn source(&self) -> &dyn Source;
}

pub trait Source {
    fn user_string(&self) -> Cow<str>;
    fn user_pretty(&self) -> Cow<str>;
    fn channel_string(&self) -> Cow<str>;

    fn get_discord_params(&self) -> Option<(Option<u64>, u64, u64)>;
    fn get_irc_params(&self) -> Option<(Option<String>, String)>;
}

#[derive(Clone)]
pub enum Message<'a> {
    Simple(String),
    Spans(Vec<Span<'a>>),
    Prefixed(Vec<Span<'a>>, Vec<Span<'a>>),
    Code(String),
    List {
        prefix: Cow<'a, str>,
        sep: Cow<'a, str>,
        items: Vec<Cow<'a, str>>,
    },
}
