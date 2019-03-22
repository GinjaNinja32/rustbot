extern crate irc;
extern crate reqwest;
extern crate rusqlite;
extern crate serenity;
#[macro_use]
extern crate bitflags;

pub mod error;
pub mod types;

pub mod prelude {
    pub use error::*;
    pub use types::Prefix::*;
    pub use types::Source::*;
    pub use types::*;
}
