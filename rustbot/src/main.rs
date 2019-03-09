extern crate irc;
extern crate libloading;
extern crate migrant_lib;
extern crate rusqlite;
extern crate serde_derive;
extern crate serenity;
extern crate shared;

mod bot;
mod db;

fn main() {
    match bot::start() {
        Ok(()) => (),
        Err(e) => println!("error: {}", e),
    }
}
