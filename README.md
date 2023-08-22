# Migratour - Simple Database Migration CLI

Migratour is a command-line tool written in Rust that facilitates database migration for both PostgreSQL and MySQL databases. It streamlines the process of managing migrations by providing commands to create, apply, and revert database changes.

### Installation

You can install Migratour using Cargo, the Rust package manager. Run the following command in your terminal -

```bash
cargo install migratour
```

### Supported Databases

Migratour supports both **PostgreSQL** and **MySQL** databases.

### Configuration

Migratour allows you to specify the database type and connection URL either through a configuration file (db.toml) or command-line options.

You can create a `db.toml` configuration file in the root of your project. Here's an example configuration for PostgreSQL database

```toml
database = "postgres" # or mysql
database_url = "postgres://username:password@hostname:port/database_name"
```

Replace the placeholders with your actual database information.Command-line Options

Alternatively, you can specify the database type and connection URL using command-line options:

`-d or  --db` : Specify the type of the database (`postgres` or `mysql`).

`-u, --db-url` : Provide the connection URL for the database.Usage

### Commands:

To create a **new migration**, use the `new` command followed by the desired migration name:

```bash
migratour new my_migration
```

This created a new migrations folder in the root of your project and also initialized a db_migrations table(to track the migration) in your database.

---

To **apply migrations**, use the `up` command along with the number of migrations you want to apply.

```bash
migratour up 3
```

This will execute the SQL scripts in the next three up migration files.

---

Reverting Migrations

If you need to **revert migrations**, you can use the `down` command along with the number of migrations you want to revert.

```bash
migratour down 1
```

This will execute the SQL scripts in the next three down migration files.
This will undo the changes made by the last applied migration.

---

To get the name of the last applied migration, use the `last` command

```bash
migratour last
```

---

To retrieve the number of migrations applied, use the `num` command:bash

```bash
migratour num
```

---


### Migrations Folder

Migratour automatically creates a `migrations` folder in your project directory. This is where all the migration SQL files are stored. Each migration has two corresponding up and down sql files, which will be executed during up and down operations respectively.






