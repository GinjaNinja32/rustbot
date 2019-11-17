use config::PostgresConfig;
use migrant_lib::config::PostgresSettingsBuilder;
use postgres::{Connection, TlsMode};

pub fn open(pc: &PostgresConfig) -> Result<Connection, String> {
    let conn_str = migrate(pc).map_err(|e| format!("{}", e))?;
    Connection::connect(conn_str, TlsMode::None).map_err(|e| format!("{}", e))
}

fn migrate(pc: &PostgresConfig) -> Result<String, Box<dyn std::error::Error>> {
    let config = migrant_lib::config::Config::with_settings(
        &PostgresSettingsBuilder::empty()
            .database_name(&pc.database)
            .database_user(&pc.user)
            .database_password(&pc.password)
            .database_host(&pc.host)
            .database_port(pc.port)
            .build()?,
    );

    config.setup()?;
    let config = config.reload()?;

    println!("Applying all migrations...");
    migrant_lib::Migrator::with_config(&config)
        .direction(migrant_lib::Direction::Up)
        .all(true)
        .apply()
        .or_else(|e| if e.is_migration_complete() { Ok(()) } else { Err(e) })?;

    let config = config.reload()?;
    migrant_lib::list(&config)?;

    return Ok(config.connect_string()?);
}
