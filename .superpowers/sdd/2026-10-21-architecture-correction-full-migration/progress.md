# SDD ledger — plan: docs/superpowers/plans/2026-10-21-architecture-correction-full-migration.md
Ruling: execute in the existing checkout — the user forbids branches and the current uncommitted migration must be preserved — cost if wrong: reduced isolation from unrelated dirty-tree changes
Pre-flight: Task 1 server adapter contracts feed Tasks 3, 7, and 8; Task 2 ownership feeds Tasks 3-6; Task 6 contracts feed Task 7.
Task 1: complete (no commits by user instruction; tests: cargo test -p aevio-persistence -p aevio-persistence-postgres -p aevio-server --no-fail-fast -> pass; bun run verify -> pass)
Task 1: Ruling: keep PostgreSQL as a nested adapter package under crates/aevio-persistence — SeaORM 2.0.4 async and sea-orm-sync 2.0.4 derive macros both require the absolute crate name sea_orm and cannot coexist in one Cargo package — cost if wrong: one additional technical Cargo package inside the single logical persistence boundary
Task 1: PostgreSQL contract binaries returned pass through their explicit skip path because AEVIO_TEST_POSTGRES_URL was not configured in this run; real PostgreSQL execution remains required by Task 10.
