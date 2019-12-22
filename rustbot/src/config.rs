use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;

use rustbot::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub postgres: PostgresConfig,

    pub irc: Vec<IRCConfig>,
    pub discord: Vec<DiscordConfig>,

    pub module: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize)]
pub struct PostgresConfig {
    pub database: String,
    pub user: String,
    pub password: String,

    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct IRCConfig {
    pub id: String,

    pub nick: String,
    pub user: String,
    pub real: String,

    pub server: String,
    pub port: u16,

    pub ssl: bool,
}

#[derive(Deserialize)]
pub struct DiscordConfig {
    pub id: String,

    pub token: String,
}

pub fn load() -> Result<Config> {
    Ok(toml::from_str(&fs::read_to_string("Rustbot.toml")?)?)
}
