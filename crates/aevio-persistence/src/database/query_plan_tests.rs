use std::time::Instant;

use rusqlite::Connection;
use sea_orm::entity::prelude::*;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

use super::entities::entry_query::entries;
use crate::bootstrap::migrations::run_migrations;

const ROWS: i64 = 50_000;
const ITERATIONS: usize = 30;

#[test]
fn typed_entry_listing_keeps_the_indexed_plan_and_benchmark_baseline() {
    let path = std::env::temp_dir().join(format!(
        "aevio-query-plan-{}-{}.db",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_file(&path);
    let mut sqlite = Connection::open(&path).expect("create query-plan fixture");
    run_migrations(&mut sqlite).expect("create schema");
    let transaction = sqlite.transaction().expect("start fixture transaction");
    for index in 0..ROWS {
        transaction
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, status, source, billable, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'approved', 'manual', 0, ?2, ?3)",
                rusqlite::params![format!("benchmark-{index}"), index * 60_000, index * 60_000 + 30_000, "Benchmark"],
            )
            .expect("seed entry");
    }
    transaction.commit().expect("commit fixture");

    let plan = sqlite
        .prepare(
            "EXPLAIN QUERY PLAN SELECT id FROM time_entries
             WHERE deleted_at IS NULL AND started_at >= 1000000 AND started_at < 2000000
             ORDER BY started_at DESC, id ASC LIMIT 100",
        )
        .expect("prepare plan")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("read plan")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect plan");
    assert!(
        plan.iter()
            .any(|detail| detail.contains("idx_time_entries_started_at")),
        "plan did not use the timestamp index: {plan:?}"
    );

    let raw_start = Instant::now();
    for _ in 0..ITERATIONS {
        let mut statement = sqlite
            .prepare(
                "SELECT id, started_at, ended_at, description, category_id, project_id, status,
                        approved_by, source, billable, invoice_id, created_at, updated_at,
                        deleted_at, description_origin FROM time_entries
                 WHERE deleted_at IS NULL AND started_at >= ?1 AND started_at < ?2
                 ORDER BY started_at DESC, id ASC LIMIT 100",
            )
            .expect("prepare raw listing");
        let count = statement
            .query_map([1_000_000_i64, 2_000_000_i64], |row| {
                let _: String = row.get(0)?;
                let _: i64 = row.get(1)?;
                let _: i64 = row.get(2)?;
                let _: String = row.get(3)?;
                let _: Option<String> = row.get(4)?;
                let _: Option<String> = row.get(5)?;
                let _: String = row.get(6)?;
                let _: Option<String> = row.get(7)?;
                let _: String = row.get(8)?;
                let _: i64 = row.get(9)?;
                let _: Option<String> = row.get(10)?;
                let _: i64 = row.get(11)?;
                let _: i64 = row.get(12)?;
                let _: Option<i64> = row.get(13)?;
                let _: String = row.get(14)?;
                Ok(())
            })
            .expect("execute raw listing")
            .count();
        assert_eq!(count, 17);
    }
    let raw_elapsed = raw_start.elapsed();

    drop(sqlite);
    let database = super::connect(&path).expect("connect SeaORM fixture");
    let typed_query = entries::Entity::find()
        .filter(entries::Column::DeletedAt.is_null())
        .filter(entries::Column::StartedAt.gte(1_000_000_i64))
        .filter(entries::Column::StartedAt.lt(2_000_000_i64))
        .order_by_desc(entries::Column::StartedAt)
        .order_by_asc(entries::Column::Id)
        .limit(100);
    let typed_start = Instant::now();
    for _ in 0..ITERATIONS {
        let rows = typed_query
            .clone()
            .all(&database)
            .expect("execute typed listing");
        assert_eq!(rows.len(), 17);
    }
    let typed_elapsed = typed_start.elapsed();
    drop(database);
    std::fs::remove_file(path).expect("remove query-plan fixture");

    println!(
        "entry listing benchmark: raw={:?}, seaorm={:?}, plan={:?}",
        raw_elapsed, typed_elapsed, plan
    );
}
