extern crate serde_derive;
extern crate toml;

use std::fs::File;
use std::io::Read;
use self::serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub cmdchars: String,
    pub modules: Vec<String>,
}

pub fn load_config()-> Config {
    let path = "conf/bot.toml";
    let mut config_toml = String::new();

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(err) => {
            panic!("failed to load config file: {}", err);
        }
    };

    file.read_to_string(&mut config_toml).unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

    toml::from_str(&config_toml).unwrap()
}
