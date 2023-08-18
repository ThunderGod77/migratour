use std::env;
use std::error::Error;
use std::i32;
use std::path::Path;
use std::process;

use db::Db;
use db::DbExe;
use db::MySqlDb;
use db::PostgresDb;
use serde::Deserialize;
use serde::Deserializer;

mod db;

use std::io::Write;

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

#[derive(Debug, Clone)]
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

pub fn read_migration_files() -> Result<Vec<String>, Box<dyn Error>> {
    let entries = fs::read_dir("./migrations")?;
    let file_names: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() {
                path.file_name()?.to_str().map(|s| s.to_owned())
            } else {
                None
            }
        })
        .collect();

    return Ok(file_names);
}

pub fn new_migration(name: &String) -> Result<(), Box<dyn Error>> {
    let mg_folder_exists = Path::new("./migrations").is_dir();

    if !mg_folder_exists {
        fs::create_dir("./migrations")?;
    }

    let file_names = read_migration_files()?;

    let file_serial_extracted: Vec<String> = file_names
        .iter()
        .map(|s| s.chars().take(4).collect())
        .collect();

    let mut valid = true;
    let serial: Vec<i32> = file_serial_extracted
        .iter()
        .map(|s| {
            s.parse::<i32>().unwrap_or_else(|_| {
                valid = false;
                -1
            })
        })
        .collect();

    if valid == false {
        return Err("invalid name for your migration files")?;
    }

    let largest = serial.iter().max();
    let largets_serial = match largest {
        Some(n) => n,
        None => &0,
    }
    .to_owned();

    let new_serial = largets_serial + 1;
    let formatted_serial = format!("{:04}", new_serial);

    let migration_name_up = "./migrations/".to_owned() + &formatted_serial + "_" + name + ".up.sql";
    let migration_name_down =
        "./migrations/".to_owned() + &formatted_serial + "_" + name + ".down.sql";

    let mut up_file = fs::File::create(migration_name_up)?;
    let mut down_file = fs::File::create(migration_name_down)?;

    up_file.write("--Please write your up migrations here".as_bytes())?;
    down_file.write("--Please write your down migrations here".as_bytes())?;

    println!("initialized migration file {}", name);
    Ok(())
}

fn filter_migration_file(mg_type: &str, mg_files: Vec<String>) -> Vec<String> {
    mg_files
        .iter()
        .filter(|file_name| {
            let ext: Vec<&str> = file_name.split(".").collect();
            if let Some(second_last_word) = ext.get(ext.len() - 2) {
                if second_last_word.to_lowercase() == mg_type {
                    return true;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        })
        .map(|s| s.to_string())
        .collect()
}

pub async fn up_migration(pool: DbExe, num: i32) -> Result<(), Box<dyn Error>> {
    let migrations_applied_num = pool.get_migration_table_count().await?;

    let migration_files = read_migration_files()?;

    let mut up_migration_files: Vec<String> = filter_migration_file("up", migration_files);

    up_migration_files.sort();

    let unapplied_migrations: Vec<&String> = up_migration_files
        .iter()
        .skip(migrations_applied_num)
        .collect();

    let migrations_to_apply: i32;
    if num == -1 {
        migrations_to_apply = unapplied_migrations.len() as i32;
    } else {
        migrations_to_apply = num;
    }

    pool.up_migration_transaction(unapplied_migrations, migrations_to_apply)
        .await?;

    Ok(())
}

pub async fn down_migration(pool: DbExe, num: i32) -> Result<(), Box<dyn Error>> {
    let migrations_applied_num = pool.get_migration_table_count().await?;

    let migration_files = read_migration_files()?;

    let mut down_migration_files: Vec<String> = filter_migration_file("down", migration_files);

    down_migration_files.sort();

    let down_migrations: Vec<&String> = down_migration_files
        .iter()
        .skip(migrations_applied_num - (num as usize))
        .take(num as usize)
        .collect();

    pool.down_migration_transaction(down_migrations).await?;

    Ok(())
}

pub async fn cmd_run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("please pass some argument");
        process::exit(1)
    }

    let mut f = Flags::parse(args).unwrap_or_else(|err| {
        eprintln!("error parsing flags {}", err);
        process::exit(1);
    });

    if f.config.database_url == "" {
        f.config = read_config_file().unwrap_or_else(|err| {
            eprintln!("error reading file {}", err);
            process::exit(1);
        });
    }

    let  db_conn: DbExe = match f.config.database {
        DatabaseType::MySql => {
            DbExe::MySqlExe(MySqlDb::new_connection(f.config.database_url).await?)
        }
        DatabaseType::Postgres => {
            DbExe::PgExe(PostgresDb::new_connection(f.config.database_url).await?)
        }
    };

    db_conn.ping_db().await.unwrap_or_else(|err| {
        eprintln!("error connecting to the database {}", err);
        process::exit(1);
    });

    let tb_exists = db_conn.table_exists().await.unwrap_or_else(|err| {
        eprintln!("error connecting to database {}", err);
        process::exit(1);
    });

    if !tb_exists {
        db_conn
            .create_migration_table()
            .await
            .unwrap_or_else(|err| {
                eprintln!("error creating database migration table {}", err);
                process::exit(1);
            })
    }

    match &f.cmd {
        Command::New(s) => new_migration(&s.clone()).unwrap_or_else(|err| {
            eprintln!("there is some error in migration files {}", err);
            process::exit(1)
        }),
        Command::Up(all, n) => {
            let num: i32;
            if *all == true {
                num = -1;
            } else {
                num = *n;
            }

            up_migration(db_conn, num).await.unwrap_or_else(|err| {
                eprintln!("there was some error when migrating up {}", err);
                process::exit(1)
            })
        }
        Command::Down(n) => down_migration(db_conn, *n).await.unwrap_or_else(|err| {
            eprintln!("there was some error when migrating down {}", err);
            process::exit(1)
        }),
    }

    Ok(())
}
