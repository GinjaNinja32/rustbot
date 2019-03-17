use irc::error as irc;
use reqwest;
use serenity::prelude::*;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub struct Error {
    msg: String,
}
impl Error {
    pub fn new(from: &str) -> Self {
        Error {
            msg: from.to_string(),
        }
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
        Error {
            msg: format!("{}", s),
        }
    }
}
impl From<String> for Error {
    fn from(s: String) -> Self {
        Error { msg: s.clone() }
    }
}
impl From<Box<std::error::Error>> for Error {
    fn from(s: Box<std::error::Error>) -> Self {
        Error {
            msg: format!("{}", s),
        }
    }
}
impl From<std::io::Error> for Error {
    fn from(s: std::io::Error) -> Self {
        Error {
            msg: format!("{}", s),
        }
    }
}
impl From<rusqlite::Error> for Error {
    fn from(s: rusqlite::Error) -> Self {
        Error {
            msg: format!("{}", s),
        }
    }
}
impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(s: std::sync::PoisonError<T>) -> Self {
        Error {
            msg: format!("{}", s),
        }
    }
}
impl From<reqwest::Error> for Error {
    fn from(s: reqwest::Error) -> Self {
        Error {
            msg: format!("{}", s),
        }
    }
}
impl From<irc::IrcError> for Error {
    fn from(s: irc::IrcError) -> Self {
        Error {
            msg: format!("{}", s),
        }
    }
}
