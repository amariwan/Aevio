use std::path::Path;

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr};

pub fn connect(path: &Path) -> Result<DatabaseConnection, DbErr> {
    let url = format!("sqlite://{}?mode=rw", path.display());
    let mut options = ConnectOptions::new(url);
    options
        .max_connections(1)
        .min_connections(1)
        .sqlx_logging(false);
    let database = Database::connect(options)?;
    database.execute_unprepared("PRAGMA foreign_keys = ON")?;
    database.execute_unprepared("PRAGMA busy_timeout = 5000")?;
    database.execute_unprepared("PRAGMA journal_mode = WAL")?;
    Ok(database)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ConnectionTrait;
    use sea_orm::sea_query::{Alias, Expr, Query, SelectExprTrait};

    #[test]
    fn connects_to_an_existing_sqlite_file_without_schema_changes() {
        let path =
            std::env::temp_dir().join(format!("aevio-seaorm-connection-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        rusqlite::Connection::open(&path)
            .expect("create sqlite fixture")
            .execute_batch("PRAGMA user_version = 25;")
            .expect("seed schema version");

        let database = connect(&path).expect("connect through SeaORM");
        drop(database);
        let version: i64 = rusqlite::Connection::open(&path)
            .expect("reopen sqlite fixture")
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 25);
        std::fs::remove_file(path).expect("remove sqlite fixture");
    }

    #[test]
    fn sync_transaction_rolls_back_without_touching_the_shared_connection() {
        let path = std::env::temp_dir().join(format!(
            "aevio-seaorm-transaction-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        rusqlite::Connection::open(&path).expect("create sqlite fixture");
        let database = connect(&path).expect("connect through SeaORM");
        database
            .execute_unprepared("CREATE TABLE transaction_probe (id INTEGER PRIMARY KEY)")
            .expect("create probe table");

        let result = database.transaction::<_, (), DbErr>(|transaction| {
            transaction.execute_unprepared("INSERT INTO transaction_probe (id) VALUES (1)")?;
            Err(DbErr::Custom("rollback probe".into()))
        });
        assert!(result.is_err());

        let count = database
            .query_one(
                &Query::select()
                    .expr(Expr::cust("COUNT(*)").alias(Alias::new("count")))
                    .from(Alias::new("transaction_probe"))
                    .to_owned(),
            )
            .expect("query probe")
            .expect("count row")
            .try_get::<i64>("", "count")
            .expect("read count");
        assert_eq!(count, 0);
        std::fs::remove_file(path).expect("remove sqlite fixture");
    }
}
