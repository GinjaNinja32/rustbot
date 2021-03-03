#![allow(non_upper_case_globals)]

use log::Level;
use parking_lot::Mutex;
use postgres::types::FromSql;
use std::borrow::Cow;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
        write!(f, "{:?}", self)?;

        let diff = self.bits & !Perms::all().bits;
        if diff != 0 {
            write!(f, " | 0x{:x}", diff)?;
        }

        Ok(())
    }
}

impl FromSql<'_> for Perms {
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
    pub fn req_perms(&self, p: Perms) -> Self {
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
    fn load_module(&self, _: &str) -> Result<()>;
    fn drop_module(&self, _: &str) -> Result<()>;
    fn set_log_level(&self, _: Level) -> Result<()>;
    fn set_module_log_level(&self, _: &str, _: Option<Level>) -> Result<()>;
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

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Color {
    None = -1,
    BrightWhite,
    Black,
    Blue,
    Green,
    BrightRed,
    Red,
    Magenta,
    Yellow,
    BrightYellow,
    BrightGreen,
    Cyan,
    BrightCyan,
    BrightBlue,
    BrightMagenta,
    BrightBlack,
    White,
}

impl From<u8> for Color {
    fn from(v: u8) -> Self {
        match v {
            0 => Color::BrightWhite,
            1 => Color::Black,
            2 => Color::Blue,
            3 => Color::Green,
            4 => Color::BrightRed,
            5 => Color::Red,
            6 => Color::Magenta,
            7 => Color::Yellow,
            8 => Color::BrightYellow,
            9 => Color::BrightGreen,
            10 => Color::Cyan,
            11 => Color::BrightCyan,
            12 => Color::BrightBlue,
            13 => Color::BrightMagenta,
            14 => Color::BrightBlack,
            15 => Color::White,
            _ => Color::None,
        }
    }
}

impl std::ops::Add<Format> for Color {
    type Output = FormatColor;

    fn add(self, rhs: Format) -> FormatColor {
        FormatColor(rhs, self)
    }
}

bitflags! {
    pub struct Format: u8 {
        const None = 0x00;
        const Bold = 0x01;
        const Italic = 0x02;
        const Underline = 0x04;
    }
}

impl std::ops::Add<Format> for Format {
    type Output = Format;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Format) -> Format {
        self | rhs
    }
}

#[derive(Copy, Clone)]
pub struct FormatColor(pub Format, pub Color);

impl std::ops::Add<Format> for FormatColor {
    type Output = FormatColor;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Format) -> FormatColor {
        FormatColor(self.0 | rhs, self.1)
    }
}

impl From<Format> for FormatColor {
    fn from(f: Format) -> Self {
        Self(f, Color::None)
    }
}

impl From<Color> for FormatColor {
    fn from(c: Color) -> Self {
        Self(Format::None, c)
    }
}

#[derive(Clone)]
pub enum Message<'a> {
    Simple(String),
    Spans(Vec<Span<'a>>),
    Prefixed(Vec<Span<'a>>, Vec<Span<'a>>),
    Code(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Span<'a> {
    Text {
        text: Cow<'a, str>,
        format: Format,
        color: Color,
        bg: Color,
    },
    DiscordEmoji(Cow<'a, str>, u64),
}

impl<'a> From<String> for Span<'a> {
    fn from(s: String) -> Self {
        span!(s)
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(s: &'a str) -> Self {
        span!(s)
    }
}

impl<'a> From<Cow<'a, str>> for Span<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        span!(s)
    }
}

use crate::span;

#[macro_export]
macro_rules! span {
    ($fc:expr; $text:expr) => {{
        let fc: $crate::types::FormatColor = $fc.into();
        $crate::types::Span::Text {
            text: $text.into(),
            format: fc.0,
            color: fc.1,
            bg: $crate::types::Color::None,
        }
    }};
    ($fc:expr; $fmt:literal, $($arg:tt)*) => {{
        let fc: $crate::types::FormatColor = $fc.into();
        $crate::types::Span::Text {
            text: format!($fmt, $($arg)*).into(),
            format: fc.0,
            color: fc.1,
            bg: $crate::types::Color::None,
        }
    }};
    ($text: expr) => { $crate::span!($crate::types::Format::None; $text) };
    ($fmt:literal, $($arg:tt)*) => { $crate::span!($crate::types::Format::None, $fmt, $($arg)*) };
}

#[macro_export]
macro_rules! spans {
    ($($x:expr),*) => {{
        let mut v: Vec<$crate::types::Span> = vec![];
        $(
            $crate::types::MoveToVecSpan::move_to_vec_span($x, &mut v);
        )*
        v
    }};
    ($($x:expr,)*) => ($crate::spans![$($x),*])
}

pub fn span_join<'a, M, C>(mut spans: Vec<M>, sep: C) -> Vec<Span<'a>>
where
    M: MoveToVecSpan<'a>,
    C: CloneToVecSpan<'a>,
{
    let mut v = vec![];
    if spans.is_empty() {
        return v;
    }
    spans.remove(0).move_to_vec_span(&mut v);
    if spans.is_empty() {
        return v;
    }

    for el in spans.drain(..) {
        sep.clone_to_vec_span(&mut v);
        el.move_to_vec_span(&mut v);
    }

    v
}

pub fn span_split<'a>(spans: Vec<Span<'a>>, sep: char) -> Vec<Vec<Span<'a>>> {
    let mut ret = vec![];
    let mut cur = vec![];

    for span in spans {
        match span {
            Span::Text {
                text,
                format,
                color,
                bg,
            } => {
                let parts = text.split(sep).collect::<Vec<_>>();
                cur.push(Span::Text {
                    text: parts[0].to_string().into(),
                    format,
                    color,
                    bg,
                });
                for part in &parts[1..] {
                    ret.push(cur);
                    cur = vec![];
                    cur.push(Span::Text {
                        text: (*part).to_string().into(),
                        format,
                        color,
                        bg,
                    });
                }
            }
            Span::DiscordEmoji { .. } => {
                cur.push(span);
            }
        }
    }

    ret.push(cur);

    ret
}

mod private {
    use crate::types::Span;

    pub trait Sealed {}

    impl<'a> Sealed for Vec<Span<'a>> {}
    impl<'a, T: Into<Span<'a>>> Sealed for T {}

    pub trait SealedClone {}

    impl<'a, T: Sealed + Clone> SealedClone for T {}
}

pub trait MoveToVecSpan<'a>: private::Sealed {
    fn move_to_vec_span(self, _: &mut Vec<Span<'a>>);
}

impl<'a> MoveToVecSpan<'a> for Vec<Span<'a>> {
    fn move_to_vec_span(mut self, v: &mut Vec<Span<'a>>) {
        v.append(&mut self);
    }
}
impl<'a, T: Into<Span<'a>>> MoveToVecSpan<'a> for T {
    fn move_to_vec_span(self, v: &mut Vec<Span<'a>>) {
        v.push(self.into());
    }
}

pub trait CloneToVecSpan<'a>: private::SealedClone {
    fn clone_to_vec_span(&self, _: &mut Vec<Span<'a>>);
}

impl<'a, T: MoveToVecSpan<'a> + Clone> CloneToVecSpan<'a> for T {
    fn clone_to_vec_span(&self, v: &mut Vec<Span<'a>>) {
        self.clone().move_to_vec_span(v);
    }
}
