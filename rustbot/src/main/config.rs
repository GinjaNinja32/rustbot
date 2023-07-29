use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;

use rustbot::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub postgres: Postgres,

    pub irc: Vec<Irc>,
    pub discord: Vec<Discord>,

    pub module: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize)]
pub struct Postgres {
    pub database: String,
    pub user: String,
    pub password: String,

    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct Irc {
    pub id: String,

    pub nick: String,
    pub user: String,
    pub real: String,

    pub server: String,
    pub port: u16,

    pub pass: Option<String>,

    pub ssl: bool,
}

#[derive(Deserialize)]
pub struct Discord {
    pub id: String,

    pub token: String,
}

pub fn load() -> Result<Config> {
    Ok(toml::from_str(&fs::read_to_string("Rustbot.toml")?)?)
}
