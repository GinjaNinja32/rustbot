#[macro_use]
extern crate bitflags;
extern crate reqwest; // reqwest being here keeps ~13MB in librustbot rather than mod_weather + rustbot
extern crate toml; // similar but much smaller

pub mod types;

pub mod prelude {
    pub use crate::types::*;
}
