#![allow(non_upper_case_globals)]

use parking_lot::Mutex;
use postgres::types::FromSql;
use postgres::Connection;
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

pub type DeinitFn = dyn FnMut(&dyn Bot) -> Result<()> + Send + Sync;

pub trait Meta {
    fn cmd(&mut self, name: &str, cmd: Command);
    fn deinit(&mut self, f: Box<DeinitFn>);
}

pub trait Bot {
    fn load_module(&self, &str) -> Result<()>;
    fn drop_module(&self, &str) -> Result<()>;
    fn sql(&self) -> &Mutex<Connection>;

    fn irc_send_privmsg(&self, &str, &str, &str) -> Result<()>;
    fn irc_send_raw(&self, &str, &str) -> Result<()>;

    fn dis_send_message(&self, &str, &str, &str, &str, bool) -> Result<()>;
}

pub trait Context {
    fn bot(&self) -> &(dyn Bot + Sync);
    fn say(&self, &str) -> Result<()>;
    fn reply(&self, Message) -> Result<()>;
    fn perms(&self) -> Result<Perms>;
    fn source_str(&self) -> String;
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Color {
    None,
    Red,
    Yellow,
    Green,
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

pub enum Message<'a> {
    Simple(String),
    Spans(Vec<Span<'a>>),
    Code(String),
}

#[derive(Clone)]
pub struct Span<'a> {
    pub text: Cow<'a, str>,
    pub format: Format,
    pub color: Color,
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

use span;

#[macro_export]
macro_rules! span {
    ($fc:expr; $text:expr) => {{
        let fc: $crate::types::FormatColor = $fc.into();
        $crate::types::Span {
            text: $text.into(),
            format: fc.0,
            color: fc.1,
        }
    }};
    ($fc:expr; $fmt:literal, $($arg:tt)*) => {{
        let fc: $crate::types::FormatColor = $fc.into();
        $crate::types::Span{
            text: format!($fmt, $($arg)*).into(),
            format: fc.0,
            color: fc.1,
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
    C: CopyToVecSpan<'a>,
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
        sep.copy_to_vec_span(&mut v);
        el.move_to_vec_span(&mut v);
    }

    v
}

mod private {
    use types::Span;

    pub trait Sealed {}

    impl<'a> Sealed for Vec<Span<'a>> {}
    impl<'a, T: Into<Span<'a>>> Sealed for T {}

    pub trait SealedCopy {}

    impl<'a, T: Sealed + Copy> SealedCopy for T {}
}

pub trait MoveToVecSpan<'a>: private::Sealed {
    fn move_to_vec_span(self, &mut Vec<Span<'a>>);
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

pub trait CopyToVecSpan<'a>: private::SealedCopy {
    fn copy_to_vec_span(&self, &mut Vec<Span<'a>>);
}

impl<'a, T: MoveToVecSpan<'a> + Copy> CopyToVecSpan<'a> for T {
    fn copy_to_vec_span(&self, v: &mut Vec<Span<'a>>) {
        self.clone().move_to_vec_span(v);
    }
}
