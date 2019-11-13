use migrant_lib::config::PostgresSettingsBuilder;
use postgres::{Connection, TlsMode};
use std::env;
use std::error::Error;

pub fn open() -> Result<Connection, String> {
    let conn_str = migrate().map_err(|e| format!("{}", e))?;
    Connection::connect(conn_str, TlsMode::None).map_err(|e| format!("{}", e))
}

fn migrate() -> Result<String, Box<Error>> {
    let dir = env::current_dir().unwrap();
    if let None = migrant_lib::search_for_settings_file(&dir) {
        migrant_lib::Config::init_in(&dir)
            .with_postgres_options(
                PostgresSettingsBuilder::empty()
                    .database_name("rustbot")
                    .database_user("rustbot")
                    .migration_location("migrations")?,
            )
            .initialize()?;
    }
    let config = match migrant_lib::search_for_settings_file(&dir) {
        None => panic!("failed to find config we just set up?"),
        Some(p) => migrant_lib::Config::from_settings_file(&p)?,
    };
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
