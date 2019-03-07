extern crate irc;
extern crate libloading;
extern crate rusqlite;
extern crate serde_derive;
extern crate shared;

mod bot;
mod config;

fn main() {
    bot::start();
}
