use std::error::Error;
use std::i32;


use serde::Deserialize;
use serde::Deserializer;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::Row;

use std::fs;

#[derive(Debug)]
pub enum DatabaseType {
    Postgres,
    MySql,
}

impl Default for DatabaseType {
    fn default() -> Self {
        DatabaseType::Postgres
    }
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
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["postgres", "mysql"],
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFileInput {
    database: Option<DatabaseType>,
    database_url: Option<String>,
}

#[derive(Debug, Default)]
pub struct Config {
    pub database: DatabaseType,
    pub database_url: String,
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

pub async fn ping_db(pool: &Pool<Postgres>) -> Result<(), Box<dyn Error>> {
    let result = sqlx::query("SELECT 1 + 1 as sum").fetch_one(pool).await?;

    let _s: i32 = result.get("sum");

    Ok(())
}

pub async fn table_exists(pool: &Pool<Postgres>) -> Result<bool, Box<dyn Error>> {
    let table_exits_sql = "SELECT EXISTS (
        SELECT FROM information_schema.tables 
        where  table_name = 'db_migrations'
        );";

    let results = sqlx::query(table_exits_sql).fetch_one(pool).await?;
    let tb_exists: bool = results.get("exists");

    dbg!(tb_exists);

    Ok(tb_exists)
}

pub async fn create_migration_table(pool: &Pool<Postgres>) -> Result<(), Box<dyn Error>> {
    let create_table_sql = "create table db_migrations(
        id serial primary key,
      name text unique,
      valid bool not null,
      created_at timestamp not null DEFAULT now(),
        deleted_at timestamp not null
    );";

    sqlx::query(create_table_sql).execute(pool).await?;

    Ok(())
}
#[derive(Debug)]
pub enum Command {
    Up(bool, i32),
    Down(i32),
    New(String),
}
impl Default for Command {
    fn default() -> Self {
        Command::New(String::default())
    }
}

#[derive(Default)]
pub struct Flags {
    pub config: Config,
    pub cmd: Command,
}

impl Flags {
    pub fn parse(args: Vec<String>) -> Result<Flags, Box<dyn Error>> {
        let mut f: Flags = Flags::default();
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-u" | "--db-url" => {
                    if i + 1 < args.len() {
                        f.config.database_url = args[i + 1].clone();
                        i = i + 1
                    }
                }
                "-d" | "--db" => {
                    if i + 1 < args.len() {
                        f.config.database_url = args[i + 1].clone();
                        i = i + 1
                    }
                }
                "new" => {
                    if i + 1 < args.len() {
                        let mig_name = args[i + 1].clone();
                       

                        f.cmd = Command::New(mig_name);
                        return Ok(f);
                    } else {
                        return Err("please mention the name of the migration file")?;
                    }
                }
                "up" => {
                    if i + 1 < args.len() {
                        match args[i + 1].clone().parse::<i32>() {
                            Ok(n) => {
                                f.cmd = Command::Up(false, n);
                                return Ok(f);
                            }
                            Err(_) => {
                                return Err("please enter a valid numeric value for up command")?;
                            }
                        }
                    } else {
                        f.cmd = Command::Up(true, -1);
                    }
                }
                "down" => {
                    if i + 1 < args.len() {
                        match args[i + 1].clone().parse::<i32>() {
                            Ok(n) => {
                                f.cmd = Command::Down(n);
                                return Ok(f);
                            }
                            Err(_) => {
                                return Err("please enter a valid numeric value for down command")?;
                            }
                        }
                    } else {
                        return Err("please enter a valid numeric value for down command")?;
                    }
                }

                _ => {
                    return Err("invalid command")?;
                }
            }

            i = i + 1;
        }

        return Ok(f);
    }
}
