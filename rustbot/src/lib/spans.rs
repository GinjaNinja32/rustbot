use super::format::{Color, Format};
use std::borrow::Cow;

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

pub use crate::span;
#[macro_export]
macro_rules! span {
    ($fc:expr; $text:expr) => {{
        let fc: $crate::format::FormatColor = $fc.into();
        $crate::spans::Span::Text {
            text: $text.into(),
            format: fc.0,
            color: fc.1,
            bg: $crate::format::Color::None,
        }
    }};
    ($fc:expr; $fmt:literal, $($arg:tt)*) => {{
        let fc: $crate::format::FormatColor = $fc.into();
        $crate::spans::Span::Text {
            text: format!($fmt, $($arg)*).into(),
            format: fc.0,
            color: fc.1,
            bg: $crate::format::Color::None,
        }
    }};
    ($text: expr) => { $crate::span!($crate::format::Format::None; $text) };
    ($fmt:literal, $($arg:tt)*) => { $crate::span!($crate::format::Format::None, $fmt, $($arg)*) };
}

pub use crate::spans;
#[macro_export]
macro_rules! spans {
    ($($x:expr),*) => {{
        let mut v: Vec<$crate::spans::Span> = vec![];
        $(
            $crate::spans::MoveToVecSpan::move_to_vec_span($x, &mut v);
        )*
        v
    }};
    ($($x:expr,)*) => ($crate::spans![$($x),*])
}

pub use crate::spans_plural;
#[macro_export]
macro_rules! spans_plural {
    ($n:expr, $base:literal) => {
        $crate::spans_plural!($n, $base, "", "s")
    };
    ($n:expr, $base:literal, $pl:literal) => {
        $crate::spans_plural!($n, $base, "", $pl)
    };
    ($n:expr, $base:literal, $sg:literal, $pl:literal) => {{
        let v = $n;
        if v == 1 {
            $crate::spans!["1 ", $base, $sg]
        } else {
            $crate::spans![format!("{} ", v), $base, $pl]
        }
    }};
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

pub fn span_split(spans: Vec<Span<'_>>, sep: char) -> Vec<Vec<Span<'_>>> {
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
                    cur = vec![Span::Text {
                        text: (*part).to_string().into(),
                        format,
                        color,
                        bg,
                    }];
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
    use super::Span;

    pub trait Sealed {}

    impl<'a> Sealed for Vec<Span<'a>> {}
    impl<'a, T: Into<Span<'a>>> Sealed for T {}

    pub trait SealedClone {}

    impl<T: Sealed + Clone> SealedClone for T {}
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
