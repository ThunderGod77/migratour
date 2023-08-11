use std::error::Error;

use std::{env, process};

use migratour::{
    create_migration_table, down_migration, new_migration, ping_db, read_config_file, table_exists,
    up_migration, Command, Flags,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    let  pool = sqlx::postgres::PgPool::connect("postgres://kshitij.360one:sVTezMu4E8YG@ep-shy-king-58645115.ap-southeast-1.aws.neon.tech/neondb?sslmode=require").await?;

    ping_db(&pool).await.unwrap_or_else(|err| {
        eprintln!("error connecting to the database {}", err);
        process::exit(1);
    });

    let tb_exists = table_exists(&pool).await.unwrap_or_else(|err| {
        eprintln!("error connecting to database {}", err);
        process::exit(1);
    });

    if !tb_exists {
        create_migration_table(&pool).await.unwrap_or_else(|err| {
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
            up_migration(&pool, num).await.unwrap_or_else(|err| {
                eprintln!("there was some error when migrating up {}", err);
                process::exit(1)
            })
        }
        Command::Down(n) => down_migration(&pool, *n).await.unwrap_or_else(|err| {
            eprintln!("there was some error when migrating down {}", err);
            process::exit(1)
        }),
    }

    Ok(())
}
