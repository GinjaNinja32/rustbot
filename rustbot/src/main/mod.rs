mod bot;
mod config;
mod context;
mod core;
mod db;
mod message;

#[cfg(test)]
mod test;

fn main() {
    match bot::start() {
        Ok(()) => start_deadlock_monitor(),
        Err(e) => Err(e).unwrap(),
    }
}

use log::warn;
use parking_lot::deadlock;
use std::{thread, time::Duration};

// Spawns a new thread that watches for deadlocks.
pub fn start_deadlock_monitor() {
    loop {
        thread::sleep(Duration::from_secs(10));
        let deadlocks = deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        warn!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            warn!("Deadlock #{}", i);
            for t in threads {
                warn!("Thread Id {:#?}", t.thread_id());
                warn!("{:#?}", t.backtrace());
            }
        }
    }
}
