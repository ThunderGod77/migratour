use std::error::Error;

use std::{env, process};
pub mod postgres_db;

use migratour::{
    down_migration, new_migration, read_config_file, up_migration, Command, Flags, PostgresDb,
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

    let mut db_conn = PostgresDb::new_connection(f.config.database_url)
        .await
        .unwrap_or_else(|err| {
            eprintln!("errot connecting to the database {}", err);
            process::exit(1);
        });

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

            up_migration(&mut db_conn, num).await.unwrap_or_else(|err| {
                eprintln!("there was some error when migrating up {}", err);
                process::exit(1)
            })
        }
        Command::Down(n) => down_migration(&mut db_conn, *n)
            .await
            .unwrap_or_else(|err| {
                eprintln!("there was some error when migrating down {}", err);
                process::exit(1)
            }),
    }

    Ok(())
}
