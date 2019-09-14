#[macro_use]
extern crate bitflags;
extern crate csv;
extern crate irc;
extern crate libloading;
extern crate migrant_lib;
extern crate parking_lot;
extern crate regex;
extern crate reqwest;
extern crate rusqlite;
extern crate serde;
extern crate serde_json;
extern crate serenity;
#[macro_use]
extern crate rental;

pub mod error;
pub mod types;

pub mod bot;
mod db;

pub mod prelude {
    pub use error::*;
    pub use types::Prefix::*;
    pub use types::Source::*;
    pub use types::*;
}
