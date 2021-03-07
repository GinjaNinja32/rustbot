#[macro_use]
extern crate bitflags;
extern crate log; // force log to live in librustbot
extern crate regex;
extern crate reqwest; // reqwest being here keeps ~13MB in librustbot rather than mod_weather + rustbot
extern crate toml; // similar but much smaller

pub mod types;

pub mod prelude {
    pub use crate::bail_user;
    pub use crate::types::*;
    pub use anyhow::Context as AnyhowContext; // would conflict with types::Context, but we just need the trait in scope here and don't care about names
    pub use anyhow::{bail, Error};
    pub use log::{debug, error, info, trace, warn};
}

// This is roughly equivalent to anyhow's bail!(), but returns a UserError inside the Error so that the user sees the message.
#[macro_export]
macro_rules! bail_user {
    ($msg:literal $(,)?) => {
      return Err($crate::types::UserError::new($msg).into())
    };
    ($fmt:literal, $($arg:tt)*) => {
        return Err($crate::types::UserError::new(format!($fmt, $($arg)*)).into())
    };
}
