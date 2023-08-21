use async_trait::async_trait;
// use futures::stream::StreamExt;
use sqlx::{Database, MySql, Pool, Postgres, Row};

use std::{error::Error, fs};

#[async_trait]
pub trait Db {
    type A: Database;
    type B: Db;

    async fn new_connection(database_url: String) -> Result<Self::B, Box<dyn Error>>;

    async fn ping_db(&self) -> Result<(), Box<dyn Error>>;

    async fn table_exists(&self) -> Result<bool, Box<dyn Error>>;

    async fn create_migration_table(&self) -> Result<(), Box<dyn Error>>;

    async fn get_migration_table_count(&self) -> Result<usize, Box<dyn Error>>;

    async fn get_last_migration(&self) -> Result<String, Box<dyn Error>>;

    async fn insert_migration(
        &self,
        name: &String,
        tx: &mut sqlx::Transaction<'_, Self::A>,
    ) -> Result<(), Box<dyn Error>>;

    async fn apply_migration(
        &self,
        migration_query: &String,
        tx: &mut sqlx::Transaction<'_, Self::A>,
    ) -> Result<(), Box<dyn Error>>;

    async fn delete_migration(
        &self,
        name: &String,
        tx: &mut sqlx::Transaction<'_, Self::A>,
    ) -> Result<(), Box<dyn Error>>;

    async fn revert_migration(
        &self,
        migration_query: &String,
        tx: &mut sqlx::Transaction<'_, Self::A>,
    ) -> Result<(), Box<dyn Error>>;

    async fn up_migration_transaction(
        &self,
        unapplied_migrations: Vec<&String>,
        migrations_to_apply: i32,
    ) -> Result<(), Box<dyn Error>>;

    async fn down_migration_transaction(
        &self,
        down_migrations: Vec<&String>,
    ) -> Result<(), Box<dyn Error>>;
}

pub struct PostgresDb {
    pub pool: Pool<Postgres>,
}

#[async_trait]
impl Db for PostgresDb {
    type A = Postgres;
    type B = PostgresDb;
    async fn new_connection(database_url: String) -> Result<PostgresDb, Box<dyn Error>> {
        let pool = sqlx::postgres::PgPool::connect(&database_url).await?;

        return Ok(PostgresDb { pool });
    }

    async fn ping_db(&self) -> Result<(), Box<dyn Error>> {
        let result = sqlx::query("SELECT 1 + 1 as sum")
            .fetch_one(&self.pool)
            .await?;

        let _s: i32 = result.get("sum");

        Ok(())
    }

    async fn table_exists(&self) -> Result<bool, Box<dyn Error>> {
        let table_exits_sql = "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            where  table_name = 'db_migrations'
            );";

        let results = sqlx::query(table_exits_sql).fetch_one(&self.pool).await?;
        let tb_exists: bool = results.get("exists");

        Ok(tb_exists)
    }

    async fn create_migration_table(&self) -> Result<(), Box<dyn Error>> {
        let create_table_sql = "create table db_migrations(
            id serial primary key,
            name text unique,
            valid bool ,
            created_at timestamp not null DEFAULT now(),
            deleted_at timestamp
        );";

        sqlx::query(create_table_sql).execute(&self.pool).await?;

        Ok(())
    }

    async fn get_migration_table_count(&self) -> Result<usize, Box<dyn Error>> {
        let result = sqlx::query("SELECT id from db_migrations")
            .fetch_all(&self.pool)
            .await?;
        let count = result.len();

        Ok(count)
    }

    async fn get_last_migration(&self) -> Result<String, Box<dyn Error>> {
        let result = sqlx::query("Select name from db_migrations order by id desc limit 1;")
            .fetch_one(&self.pool)
            .await?;

        let name = result.try_get("name")?;

        Ok(name)
    }

    // async fn new_transaction(&self) -> Result<sqlx::Transaction<'_, Postgres>, Box<dyn Error>> {
    //     let tx: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;
    //     Ok(tx)
    // }

    async fn insert_migration(
        &self,
        name: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("INSERT INTO db_migrations(name, valid) VALUES ($1, $2);")
            .bind(&name)
            .bind(true)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    async fn apply_migration(
        &self,
        migration_query: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        let queries: Vec<&str> = migration_query.split(';').collect();

        for query in queries {
            sqlx::query(query).execute(&mut **tx).await?;
        }

        Ok(())
    }

    async fn delete_migration(
        &self,
        name: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("DELETE from db_migrations where name = $1;")
            .bind(name)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    async fn revert_migration(
        &self,
        migration_query: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        let queries: Vec<&str> = migration_query.split(';').collect();

        for query in queries {
            sqlx::query(query).execute(&mut **tx).await?;
        }

        Ok(())
    }

    async fn up_migration_transaction(
        &self,
        unapplied_migrations: Vec<&String>,
        migrations_to_apply: i32,
    ) -> Result<(), Box<dyn Error>> {
        let mut tx: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;

        for i in 0..migrations_to_apply {
            let mg = unapplied_migrations[i as usize];
            let mut name: String = mg.chars().skip(5).collect();
            name = name.trim_end_matches(".up.sql").to_string();

            let migration_query = fs::read_to_string("./migrations/".to_owned() + mg)?;

            if let Err(e) = self.insert_migration(&name, &mut tx).await {
                return Err(format!("error when inserting to migration {}, {}", name, e))?;
            }

            match self.apply_migration(&migration_query, &mut tx).await {
                Ok(_) => {
                    println!("applied migration {}", name)
                }
                Err(e) => {
                    return Err(format!("error when migrating {}, {}", name, e))?;
                }
            }
        }

        tx.commit().await?;

        Ok(())
    }

    async fn down_migration_transaction(
        &self,
        down_migrations: Vec<&String>,
    ) -> Result<(), Box<dyn Error>> {
        let mut tx = self.pool.begin().await?;

        for i in (0..down_migrations.len()).rev() {
            let mg = down_migrations[i as usize];
            let mut name: String = mg.chars().skip(5).collect();
            name = name.trim_end_matches(".down.sql").to_string();

            let migration_query = fs::read_to_string("./migrations/".to_owned() + mg)?;

            match self.delete_migration(&name, &mut tx).await {
                Ok(_) => {}
                Err(err) => {
                    return Err(err.into());
                }
            }

            match self.revert_migration(&migration_query, &mut tx).await {
                Ok(_) => {
                    println!("removed migration {}", name)
                }
                Err(err) => {
                    return Err(format!("error when migrating {}, {}", name, err))?;
                }
            }
        }

        tx.commit().await?;

        Ok(())
    }
}

pub struct MySqlDb {
    pub pool: Pool<MySql>,
}

#[async_trait]
impl Db for MySqlDb {
    type A = MySql;
    type B = MySqlDb;
    async fn new_connection(database_url: String) -> Result<MySqlDb, Box<dyn Error>> {
        let pool = sqlx::mysql::MySqlPool::connect(&database_url).await?;

        return Ok(MySqlDb { pool });
    }

    async fn ping_db(&self) -> Result<(), Box<dyn Error>> {
        let result = sqlx::query("SELECT 1 + 1 as sum")
            .fetch_one(&self.pool)
            .await?;

        let _s: i32 = result.get("sum");

        Ok(())
    }

    async fn table_exists(&self) -> Result<bool, Box<dyn Error>> {
        let table_exits_sql = "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            where  table_name = 'db_migrations'
            );";

        let results = sqlx::query(table_exits_sql).fetch_one(&self.pool).await?;
        let tb_exists: bool = results.get("exists");

        Ok(tb_exists)
    }

    async fn create_migration_table(&self) -> Result<(), Box<dyn Error>> {
        let create_table_sql = "create table db_migrations(
            id serial primary key,
            name text unique,
            valid bool ,
            created_at timestamp not null DEFAULT now(),
            deleted_at timestamp
        );";

        sqlx::query(create_table_sql).execute(&self.pool).await?;

        Ok(())
    }

    async fn get_migration_table_count(&self) -> Result<usize, Box<dyn Error>> {
        let result = sqlx::query("SELECT id from db_migrations")
            .fetch_all(&self.pool)
            .await?;
        let count = result.len();

        Ok(count)
    }

    async fn get_last_migration(&self) -> Result<String, Box<dyn Error>> {
        let result = sqlx::query("Select name from db_migrations order by id desc limit 1;")
            .fetch_one(&self.pool)
            .await?;

        let name = result.try_get("name")?;

        Ok(name)
    }

    // async fn new_transaction(&self) -> Result<sqlx::Transaction<'_, Postgres>, Box<dyn Error>> {
    //     let tx: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;
    //     Ok(tx)
    // }

    async fn insert_migration(
        &self,
        name: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("INSERT INTO db_migrations(name, valid) VALUES ($1, $2);")
            .bind(&name)
            .bind(true)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    async fn apply_migration(
        &self,
        migration_query: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    ) -> Result<(), Box<dyn Error>> {
        let queries: Vec<&str> = migration_query.split(';').collect();

        for query in queries {
            sqlx::query(query).execute(&mut **tx).await?;
        }

        Ok(())
    }

    async fn delete_migration(
        &self,
        name: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("DELETE from db_migrations where name = $1;")
            .bind(name)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    async fn revert_migration(
        &self,
        migration_query: &String,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    ) -> Result<(), Box<dyn Error>> {
        let queries: Vec<&str> = migration_query.split(';').collect();

        for query in queries {
            sqlx::query(query).execute(&mut **tx).await?;
        }

        Ok(())
    }

    async fn up_migration_transaction(
        &self,
        unapplied_migrations: Vec<&String>,
        migrations_to_apply: i32,
    ) -> Result<(), Box<dyn Error>> {
        let mut tx: sqlx::Transaction<'_, MySql> = self.pool.begin().await?;

        for i in 0..migrations_to_apply {
            let mg = unapplied_migrations[i as usize];
            let mut name: String = mg.chars().skip(5).collect();
            name = name.trim_end_matches(".up.sql").to_string();

            let migration_query = fs::read_to_string("./migrations/".to_owned() + mg)?;

            if let Err(e) = self.insert_migration(&name, &mut tx).await {
                return Err(format!("error when inserting to migration {}, {}", name, e))?;
            }

            match self.apply_migration(&migration_query, &mut tx).await {
                Ok(_) => {
                    println!("applied migration {}", name)
                }
                Err(e) => {
                    return Err(format!("error when migrating {}, {}", name, e))?;
                }
            }
        }

        tx.commit().await?;

        Ok(())
    }

    async fn down_migration_transaction(
        &self,
        down_migrations: Vec<&String>,
    ) -> Result<(), Box<dyn Error>> {
        let mut tx = self.pool.begin().await?;

        for i in (0..down_migrations.len()).rev() {
            let mg = down_migrations[i as usize];
            let mut name: String = mg.chars().skip(5).collect();
            name = name.trim_end_matches(".down.sql").to_string();

            let migration_query = fs::read_to_string("./migrations/".to_owned() + mg)?;

            match self.delete_migration(&name, &mut tx).await {
                Ok(_) => {}
                Err(err) => {
                    return Err(err.into());
                }
            }

            match self.revert_migration(&migration_query, &mut tx).await {
                Ok(_) => {
                    println!("removed migration {}", name)
                }
                Err(err) => {
                    return Err(format!("error when migrating {}, {}", name, err))?;
                }
            }
        }

        tx.commit().await?;

        Ok(())
    }
}

pub enum DbExe {
    PgExe(PostgresDb),
    MySqlExe(MySqlDb),
}

impl DbExe {
    pub async fn ping_db(&self) -> Result<(), Box<dyn Error>> {
        match self {
            DbExe::PgExe(pg) => pg.ping_db().await?,
            DbExe::MySqlExe(m) => m.ping_db().await?,
        }

        Ok(())
    }

    pub async fn table_exists(&self) -> Result<bool, Box<dyn Error>> {
        let tb_exists = match self {
            DbExe::PgExe(pg) => pg.table_exists().await?,
            DbExe::MySqlExe(m) => m.table_exists().await?,
        };

        Ok(tb_exists)
    }

    pub async fn create_migration_table(&self) -> Result<(), Box<dyn Error>> {
        match self {
            DbExe::PgExe(pg) => pg.create_migration_table().await?,
            DbExe::MySqlExe(m) => m.create_migration_table().await?,
        }

        Ok(())
    }

    pub async fn get_migration_table_count(&self) -> Result<usize, Box<dyn Error>> {
        let count = match self {
            DbExe::PgExe(pg) => pg.get_migration_table_count().await?,
            DbExe::MySqlExe(m) => m.get_migration_table_count().await?,
        };

        Ok(count)
    }

    // async fn new_transaction(&self) -> Result<sqlx::Transaction<'_, Postgres>, Box<dyn Error>> {
    //     let tx: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;
    //     Ok(tx)
    // }

    pub async fn up_migration_transaction(
        &self,
        unapplied_migrations: Vec<&String>,
        migrations_to_apply: i32,
    ) -> Result<(), Box<dyn Error>> {
        if unapplied_migrations.len() < migrations_to_apply as usize {
            return Err(format!(
                "unapplied migrations {} is less than migrations {} to apply",
                unapplied_migrations.len(),
                migrations_to_apply
            ))?;
        }

        match self {
            DbExe::MySqlExe(m) => {
                m.up_migration_transaction(unapplied_migrations, migrations_to_apply)
                    .await?;
            }
            DbExe::PgExe(pg) => {
                pg.up_migration_transaction(unapplied_migrations, migrations_to_apply)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn get_last_migration(&self) -> Result<String, Box<dyn Error>> {
        let name = match self {
            DbExe::MySqlExe(m) => m.get_last_migration().await?,
            DbExe::PgExe(pg) => pg.get_last_migration().await?,
        };

        Ok(name)
    }

    pub async fn down_migration_transaction(
        &self,
        down_migrations: Vec<&String>,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            DbExe::MySqlExe(m) => m.down_migration_transaction(down_migrations).await?,
            DbExe::PgExe(pg) => pg.down_migration_transaction(down_migrations).await?,
        }
        Ok(())
    }
}
