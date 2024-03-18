use std::borrow::Cow;

use super::prelude::*;

#[macro_export]
macro_rules! parse_args {
    ($args:ident, $(
        $name:ident: $ty:ty,
    )*) => {
        let ($($name),*) = match <($($ty),*) as Arg>::parse_full_no_pfx($args) {
            Ok(v) => v,
            Err(e) => return Err(UserError::new(format!("parsing {}: {}",
                format!("({})", [$(
                    format!("{}: {}", stringify!($name), <$ty as Arg>::describe_expected())
                ),*].join(", ")),
                e,
            )).into())
        };
    }
}
pub use crate::parse_args;

pub trait Arg<'a>: Sized {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)>;

    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    fn parse_full<'s: 'a>(input: &'s str) -> Result<Self> {
        Self::parse_full_no_pfx::<'s>(input)
            .map_err(|e| UserError::new(format!("parsing {}: {}", Self::describe_expected(), e)).into())
    }

    fn parse_full_no_pfx<'s: 'a>(input: &'s str) -> Result<Self> {
        let (val, rest) = Self::parse_from::<'s>(input)?;
        if let Some(rest) = rest {
            bail_user!("extra arguments at end: {:?}", rest)
        } else {
            Ok(val)
        }
    }
}

// Simple types: anything whitespace-terminated
macro_rules! ws_terminated {
    ($ty:ty, |$n:ident| $conv:expr) => {
        impl<'a> Arg<'a> for $ty {
            fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
                if input == "" {
                    bail_user!("missing argument")
                }
                let ($n, rest) = match input.split_once(char::is_whitespace) {
                    Some((this, rest)) => (this, Some(rest)),
                    None => (input, None),
                };

                Ok(($conv, rest))
            }
        }
    };
}

macro_rules! impl_ws_with_parse {
    ($( $ty:ty ),*) => {
        $(
            ws_terminated!($ty, |this| match this.parse() {
                Ok(v) => v,
                Err(e) => bail_user!("failed to parse {:?} as {}: {}", this, stringify!($ty), e)
            });
        )*
    };
}

impl_ws_with_parse! {
    i8, i16, i32, i64, isize,
    u8, u16, u32, u64, usize,
    bool
}

// More complex: strings

// Atom: a single non-quoted segment
#[derive(Debug, PartialEq, Eq)]
pub struct Atom(pub String);
impl std::ops::Deref for Atom {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl<'a> Arg<'a> for Atom {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
        if input.is_empty() {
            bail_user!("missing argument")
        }

        let (this, rest) = match input.split_once(char::is_whitespace) {
            Some((this, rest)) => (this, Some(rest)),
            None => (input, None),
        };

        Ok((Atom(this.to_string()), rest))
    }

    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed("atom")
    }
}

// String: a possibly-quoted segment
impl<'a> Arg<'a> for String {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
        if input.is_empty() {
            bail_user!("missing argument")
        }

        let (this, rest) = if &input[0..1] == "\"" {
            // Quoted string
            match input[1..].split_once('"') {
                Some((this, rest)) => {
                    if rest == "" {
                        (this, None)
                    } else if rest.chars().next().unwrap().is_whitespace() {
                        (this, Some(&rest[1..]))
                    } else {
                        bail_user!("bad quoted string");
                    }
                }
                _ => bail_user!("bad quoted string"),
            }
        } else {
            // Non-quoted string; take everything up to the next whitespace
            match input.split_once(char::is_whitespace) {
                Some((this, rest)) => (this, Some(rest)),
                None => (input, None),
            }
        };

        Ok((this.into(), rest))
    }

    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed("string")
    }
}


// String: a possibly-quoted segment
impl<'a> Arg<'a> for &'a str {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
        if input.is_empty() {
            bail_user!("missing argument")
        }

        let (this, rest) = if &input[0..1] == "\"" {
            // Quoted string
            match input[1..].split_once('"') {
                Some((this, rest)) => {
                    if rest == "" {
                        (this, None)
                    } else if rest.chars().next().unwrap().is_whitespace() {
                        (this, Some(&rest[1..]))
                    } else {
                        bail_user!("bad quoted string");
                    }
                }
                _ => bail_user!("bad quoted string"),
            }
        } else {
            // Non-quoted string; take everything up to the next whitespace
            match input.split_once(char::is_whitespace) {
                Some((this, rest)) => (this, Some(rest)),
                None => (input, None),
            }
        };

        Ok((this.into(), rest))
    }

    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed("string")
    }
}

// Rest: the rest of the input
#[derive(Debug, PartialEq, Eq)]
pub struct Rest(pub String);
impl std::ops::Deref for Rest {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl<'a> Arg<'a> for Rest {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
        if input.is_empty() {
            bail_user!("missing argument")
        }

        Ok((Rest(input.to_string()), None))
    }

    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed("rest-of-input")
    }
}

// Combining args: tuples
macro_rules! tuple_impls {
    ( $head:ident, $($tail:ident,)* ) => {
        impl<'a, $head: Arg<'a>, $( $tail:Arg<'a> ),*> Arg<'a> for ($head, $($tail),*) {
            fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
                #![allow(non_snake_case)]
                let ($head, input) = $head::parse_from(input)?;
                $(
                    let ($tail, input) = if let Some(input) = input {
                        $tail::parse_from(input)?
                    } else {
                        bail_user!("missing argument")
                    };
                )*

                Ok((($head, $($tail),*), input))
            }

            fn describe_expected() -> Cow<'static, str> {
                Cow::Owned(format!("({})", [
                    $head::describe_expected(),
                    $($tail::describe_expected()),*
                ].join(", ")))
            }
        }

        tuple_impls!($($tail,)*);
    };

    () => {};
}

tuple_impls!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, X, Y, Z,);
impl<'a> Arg<'a> for () {
    fn parse_from<'s: 'a>(_: &'s str) -> Result<(Self, Option<&'s str>)> {
        bail_user!("no arguments expected")
    }
    fn describe_expected() -> Cow<'static, str> {
        Cow::Borrowed("none")
    }
}

// Optional args: Option<T>
impl<'a, T: Arg<'a>> Arg<'a> for Option<T> {
    fn parse_from<'s: 'a>(input: &'s str) -> Result<(Self, Option<&'s str>)> {
        if let Ok((this, rest)) = T::parse_from(input) {
            Ok((Some(this), rest))
        } else {
            Ok((None, Some(input)))
        }
    }

    fn describe_expected() -> Cow<'static, str> {
        Cow::Owned(format!("optional {}", T::describe_expected()))
    }
}
