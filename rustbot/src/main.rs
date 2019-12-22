extern crate irc;
extern crate libloading;
extern crate migrant_lib;
extern crate parking_lot;
extern crate postgres;
extern crate regex;
extern crate rustbot;
extern crate serde;
extern crate serde_json;
extern crate serenity;
#[macro_use]
extern crate rental;

mod bot;
mod config;
mod context;
mod db;
mod message;

fn main() {
    match bot::start() {
        Ok(()) => start_deadlock_monitor(),
        Err(e) => Err(e).unwrap(),
    }
}

use parking_lot::deadlock;
use std::{thread, time::Duration};

// Spawns a new thread that watches for deadlocks.
pub fn start_deadlock_monitor() {
    /*
    println!("Starting deadlock monitor.");
    thread::Builder::new()
        .name("thread monitor".to_owned())
        .spawn(move ||*/
    loop {
        thread::sleep(Duration::from_secs(10));
        let deadlocks = deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        println!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            println!("Deadlock #{}", i);
            for t in threads {
                println!("Thread Id {:#?}", t.thread_id());
                println!("{:#?}", t.backtrace());
            }
        }
    }
    /*
    )
    .unwrap();
    */
}
