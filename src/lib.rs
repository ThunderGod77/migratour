use std:: error::Error;

use serde::Deserialize;
use serde::Deserializer;

use std::fs;

#[derive(Debug)]
pub enum DatabaseType {
    Postgres,
    MySql,
}

impl<'de> Deserialize<'de> for DatabaseType {
    fn deserialize<D>(deserializer: D) -> Result<DatabaseType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "postgres" => Ok(DatabaseType::Postgres),
            "mysql" => Ok(DatabaseType::MySql),
            _ => Err(serde::de::Error::unknown_variant(&s, &["postgres", "mysql"])),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFileInput {
    database: Option<DatabaseType>,
    database_url: Option<String>,
}


#[derive(Debug)]
pub struct Config {
    database: DatabaseType,
    database_url: String,
}

impl Config {
    fn new(database: DatabaseType, database_url: String) -> Config {
        Config {
            database,
            database_url,
        }
    }
}

pub fn read_config_file() -> Result<Config, Box<dyn Error>> {
    let content = fs::read_to_string("./db.toml")?;
    let decoded: ConfigFileInput = toml::from_str(&content)?;

    let db = match decoded.database {
        None => {
            return Err("bad database type name")?;
        }
        Some(a) => a,
    };

    let db_url = match decoded.database_url {
        None => {
            return Err("bad database url")?;
        }
        Some(a) => a,
    };

    return Ok(Config::new(db, db_url));
}
