use migrant_lib::config::SqliteSettingsBuilder;
use rusqlite::Connection;
use std::env;
use std::error::Error;

pub fn open() -> Result<Connection, String> {
    migrate().map_err(|e| format!("{}", e))?;
    Connection::open("bot.db").map_err(|e| format!("{}", e))
}

fn migrate() -> Result<(), Box<Error>> {
    let dir = env::current_dir().unwrap();
    if let None = migrant_lib::search_for_settings_file(&dir) {
        migrant_lib::Config::init_in(&dir)
            .with_sqlite_options(
                SqliteSettingsBuilder::empty()
                    .database_path("bot.db")?
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
    Ok(())
}
