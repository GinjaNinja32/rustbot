use irc::error as irc;
use serenity::prelude::*;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub struct Error {
    msg: String,
}
impl Error {
    pub fn new(from: &str) -> Self {
        Error { msg: from.to_string() }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.msg.as_str()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<SerenityError> for Error {
    fn from(s: SerenityError) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<String> for Error {
    fn from(s: String) -> Self {
        Error { msg: s.clone() }
    }
}
impl From<std::string::FromUtf8Error> for Error {
    fn from(s: std::string::FromUtf8Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<std::str::Utf8Error> for Error {
    fn from(s: std::str::Utf8Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<Box<std::error::Error>> for Error {
    fn from(s: Box<std::error::Error>) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<std::io::Error> for Error {
    fn from(s: std::io::Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<rusqlite::Error> for Error {
    fn from(s: rusqlite::Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(s: std::sync::PoisonError<T>) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<reqwest::Error> for Error {
    fn from(s: reqwest::Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<irc::IrcError> for Error {
    fn from(s: irc::IrcError) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<regex::Error> for Error {
    fn from(s: regex::Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<serde_json::Error> for Error {
    fn from(s: serde_json::Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<csv::Error> for Error {
    fn from(s: csv::Error) -> Self {
        Error { msg: format!("{}", s) }
    }
}
impl From<std::num::ParseIntError> for Error {
    fn from(s: std::num::ParseIntError) -> Self {
        Error { msg: format!("{}", s) }
    }
}
