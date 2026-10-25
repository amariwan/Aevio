//! Contract tests for the versioned PostgreSQL schema migrations.
//!
//! Every test runs in its own PostgreSQL schema, selected through the
//! connection's `search_path`, so the shared integration database is never
//! truncated or dropped. Without `AEVIO_TEST_POSTGRES_URL` each test records an
//! explicit skip instead of a result it did not obtain.

mod support;

use std::env;

use aevio_server::app::core::database::postgres as database;
use aevio_server::app::core::database::postgres::migrations::MIGRATIONS;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use uuid::Uuid;

use support::skipped;

const TABLES: [&str; 10] = [
    "accounts",
    "access_tokens",
    "workspaces",
    "workspace_memberships",
    "sync_operations",
    "plugin_registry",
    "plugin_registry_releases",
    "plugin_publisher_keys",
    "plugin_workspace_desired_state",
    "schema_migrations",
];

struct Schema {
    admin: DatabaseConnection,
    name: String,
    url: String,
}

impl Schema {
    /// A disposable schema and a URL whose connections resolve into it.
    async fn open(test: &str) -> Option<Self> {
        let base = env::var("AEVIO_TEST_POSTGRES_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let admin = database::connect(&base).await.expect("connect as admin");
        let name = format!("migration_{}", Uuid::now_v7().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{name}\""))
            .await
            .expect("create isolated schema");
        let separator = if base.contains('?') { '&' } else { '?' };
        let url = format!("{base}{separator}options=-c%20search_path%3D{name}");
        if let Err(error) = database::connect(&url).await {
            eprintln!("no result for {test}: schema-scoped connection failed: {error}");
            let _ = admin
                .execute_unprepared(&format!("DROP SCHEMA IF EXISTS \"{name}\" CASCADE"))
                .await;
            return None;
        }
        Some(Self { admin, name, url })
    }

    async fn connection(&self) -> DatabaseConnection {
        database::connect(&self.url)
            .await
            .expect("connect inside the isolated schema")
    }

    async fn close(self) {
        let _ = self
            .admin
            .execute_unprepared(&format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.name))
            .await;
    }
}

async fn applied(db: &DatabaseConnection) -> Vec<i64> {
    let rows = db
        .query_all_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT version FROM schema_migrations ORDER BY version",
        ))
        .await
        .expect("read the migration ledger");
    rows.iter()
        .map(|row| row.try_get::<i64>("", "version").expect("version column"))
        .collect()
}

async fn table_count(db: &DatabaseConnection) -> i64 {
    let statement = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM information_schema.tables
         WHERE table_schema = current_schema() AND table_name = ANY($1)",
        [sea_orm::Value::from(
            TABLES
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>(),
        )],
    );
    db.query_one_raw(statement)
        .await
        .expect("query information_schema")
        .expect("one count row")
        .try_get::<i64>("", "count")
        .expect("count column")
}

#[tokio::test]
async fn a_fresh_schema_applies_every_migration_once() {
    let Some(schema) = Schema::open("a_fresh_schema_applies_every_migration_once").await else {
        skipped("a_fresh_schema_applies_every_migration_once");
        return;
    };
    let db = schema.connection().await;

    database::initialize(&db).await.expect("apply migrations");
    let expected: Vec<i64> = MIGRATIONS.iter().map(|item| item.version).collect();
    assert_eq!(applied(&db).await, expected);
    assert_eq!(
        table_count(&db).await,
        TABLES.len() as i64,
        "every schema table exists"
    );

    database::initialize(&db).await.expect("second initialize");
    assert_eq!(
        applied(&db).await,
        expected,
        "re-running does not duplicate or skip a migration"
    );

    schema.close().await;
}

#[tokio::test]
async fn an_unversioned_deployment_is_adopted_without_losing_rows() {
    let Some(schema) =
        Schema::open("an_unversioned_deployment_is_adopted_without_losing_rows").await
    else {
        skipped("an_unversioned_deployment_is_adopted_without_losing_rows");
        return;
    };
    let db = schema.connection().await;
    // The state before the ledger existed: tables already created by the old
    // additive bootstrap, with real rows.
    db.execute_unprepared(MIGRATIONS[0].sql)
        .await
        .expect("seed the legacy schema");
    db.execute_unprepared(
        "INSERT INTO workspaces (id, name, created_at) VALUES ('w-legacy', 'Legacy', 1)",
    )
    .await
    .expect("seed a workspace");

    database::initialize(&db).await.expect("adopt the schema");

    assert_eq!(applied(&db).await, vec![1]);
    let rows = db
        .query_all_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT name FROM workspaces WHERE id = 'w-legacy'",
        ))
        .await
        .expect("read the workspace");
    assert_eq!(rows.len(), 1, "existing rows survive the adoption");

    schema.close().await;
}

#[tokio::test]
async fn concurrent_initializations_serialize_on_the_advisory_lock() {
    let Some(schema) =
        Schema::open("concurrent_initializations_serialize_on_the_advisory_lock").await
    else {
        skipped("concurrent_initializations_serialize_on_the_advisory_lock");
        return;
    };
    let first = schema.connection().await;
    let second = schema.connection().await;

    let (left, right) = tokio::join!(database::initialize(&first), database::initialize(&second));
    left.expect("first initialize");
    right.expect("second initialize");

    let expected: Vec<i64> = MIGRATIONS.iter().map(|item| item.version).collect();
    assert_eq!(
        applied(&first).await,
        expected,
        "concurrent starts record each version exactly once"
    );

    schema.close().await;
}
