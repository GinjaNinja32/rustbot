use crate::config;
use migrant_lib::config::PostgresSettingsBuilder;
use postgres::{Client, NoTls};

use rustbot::prelude::*;

pub fn open(pc: &config::Postgres) -> Result<Client> {
    let conn_str = migrate(pc)?;
    let conn = Client::connect(&conn_str, NoTls)?;
    Ok(conn)
}

fn migrate(pc: &config::Postgres) -> Result<String> {
    let config = migrant_lib::config::Config::with_settings(
        &PostgresSettingsBuilder::empty()
            .database_name(&pc.database)
            .database_user(&pc.user)
            .database_password(&pc.password)
            .database_host(&pc.host)
            .database_port(pc.port)
            .build()
            .map_err(from_migrant)?,
    );

    config.setup().map_err(from_migrant)?;
    let config = config.reload().map_err(from_migrant)?;

    info!("Applying all migrations...");
    migrant_lib::Migrator::with_config(&config)
        .direction(migrant_lib::Direction::Up)
        .all(true)
        .apply()
        .or_else(|e| if e.is_migration_complete() { Ok(()) } else { Err(e) })
        .map_err(from_migrant)?;

    let config = config.reload().map_err(from_migrant)?;
    migrant_lib::list(&config).map_err(from_migrant)?;

    config.connect_string().map_err(from_migrant)
}

fn from_migrant(e: migrant_lib::Error) -> Error {
    Error::msg(format!("{e}"))
}
