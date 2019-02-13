extern crate shared;
extern crate irc;
extern crate libloading;
extern crate serde_derive;

mod bot;
mod config;

fn main() {
    bot::start();
}

