extern crate shared;
extern crate irc;
extern crate libloading;

mod bot;
mod config;

fn main() {
    bot::start();
}

