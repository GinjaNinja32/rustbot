#[macro_use]
extern crate bitflags;
extern crate regex;
#[macro_use]
extern crate nom;

// These may or may not be used in librustbot itself, but they're each used by one or more modules;
// these these declarations force the compiler to put them in librustbot.so rather than duplicating
// them across each module they're used in
extern crate log;
extern crate reqwest;
extern crate toml;

// These are pub so that async_thread!{} can $crate::... them, without needing to put them in
// each module's Cargo.toml.
pub extern crate futures;
pub extern crate tokio;

pub mod types;
pub mod utils;

#[cfg(test)]
mod libtest;

pub mod prelude {
    pub use crate::bail_user;
    pub use crate::thread;
    pub use crate::types::*;
    pub use crate::utils::*;
    pub use anyhow::Context as AnyhowContext; // would conflict with types::Context, but we just need the trait in scope here and don't care about names
    pub use anyhow::{anyhow, bail, Error};
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

#[macro_export]
macro_rules! thread {
    ($meta:ident, async $code:block) => {{
        let unload = $crate::types::Meta::on_unload_channel($meta);
        $meta.thread(Box::new(|| {
            let rt = $crate::tokio::runtime::Runtime::new().unwrap();
            let res: $crate::types::Result<()> = rt
                .block_on(async {
                    let s = async { $code };

                    $crate::futures::select! {
                        r = $crate::futures::future::FutureExt::fuse(s) => r,
                        _ = $crate::futures::future::FutureExt::fuse(unload) => Ok(()),
                    }
                })
                .map_err(::std::convert::Into::into);
            res.unwrap();
        }));
    }};
}
