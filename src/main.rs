
use std::error::Error;
use std::{env, process};

use migratour::{ping_db, read_config_file};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let config = read_config_file().unwrap_or_else(|err| {
        eprintln!("error reading file {}", err);
        process::exit(1);
    });

    let mut pool = sqlx::postgres::PgPool::connect("postgres://kshitij.360one:sVTezMu4E8YG@ep-shy-king-58645115.ap-southeast-1.aws.neon.tech/neondb?sslmode=require").await?;

    

    ping_db(&pool).await?;

    dbg!(config);

    Ok(())
}
