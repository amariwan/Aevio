use super::*;

#[derive(Debug, Clone)]
pub struct Artifact {
    pub id: String,
    pub path: String,
}

pub fn active_artifact(conn: &Connection, kind: &str) -> Result<Option<Artifact>> {
    conn.query_row(
        "SELECT id, path FROM model_artifacts
         WHERE kind = ?1 AND active = 1 ORDER BY trained_at DESC LIMIT 1;",
        params![kind],
        |row| {
            Ok(Artifact {
                id: row.get(0)?,
                path: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(err)
}

/// Every trained version is recorded, including ones that lost the holdout
/// comparison (`active = 0`), so Settings can show why a retrain didn't swap.
pub fn insert_artifact(
    conn: &Connection,
    kind: &str,
    path: &str,
    n_examples: u32,
    holdout_acc: Option<f64>,
    active: bool,
    now: u64,
) -> Result<()> {
    if active {
        conn.execute(
            "UPDATE model_artifacts SET active = 0 WHERE kind = ?1;",
            params![kind],
        )
        .map_err(err)?;
    }
    conn.execute(
        "INSERT INTO model_artifacts (id, kind, path, trained_at, n_examples, holdout_acc, active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
        params![
            new_id(),
            kind,
            path,
            now as i64,
            n_examples,
            holdout_acc,
            active as i64
        ],
    )
    .map_err(err)?;
    prune_artifacts(conn, kind)
}

/// Drops all but the newest `ARTIFACT_HISTORY` inactive versions of a kind.
/// Their model files are already gone: the worker deletes a model's
/// directory as soon as it stops being active.
pub(crate) fn prune_artifacts(conn: &Connection, kind: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM model_artifacts WHERE kind = ?1 AND active = 0 AND id NOT IN (
           SELECT id FROM model_artifacts WHERE kind = ?1 AND active = 0
           ORDER BY trained_at DESC LIMIT ?2
         );",
        params![kind, ARTIFACT_HISTORY],
    )
    .map(|_| ())
    .map_err(err)
}

/// A trained version as Settings shows it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersion {
    pub id: String,
    pub trained_at: u64,
    pub examples: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout_accuracy: Option<f64>,
    pub active: bool,
}

/// Newest first.
pub fn artifact_history(conn: &Connection, kind: &str, limit: u32) -> Result<Vec<ArtifactVersion>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, trained_at, n_examples, holdout_acc, active FROM model_artifacts
             WHERE kind = ?1 ORDER BY trained_at DESC, active DESC LIMIT ?2;",
        )
        .map_err(err)?;
    let rows = stmt
        .query_map(params![kind, limit], |row| {
            Ok(ArtifactVersion {
                id: row.get(0)?,
                trained_at: row.get::<_, i64>(1)? as u64,
                examples: row.get::<_, i64>(2)? as u32,
                holdout_accuracy: row.get(3)?,
                active: row.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(err)?;
    rows.collect::<rusqlite::Result<_>>().map_err(err)
}

/// (entry id, content text, label) for every approved entry with a vector.
/// Project rows use `none` for "No project".
pub fn training_rows(conn: &Connection, field: Field) -> Result<Vec<(String, String, String)>> {
    let column = match field {
        Field::Category => "e.category_id",
        Field::Project => "COALESCE(e.project_id, 'none')",
    };
    let mut stmt = conn
        .prepare(&format!(
            "SELECT e.id, v.features, {column} FROM time_entries e
             JOIN entry_embeddings v ON v.entry_id = e.id
             WHERE e.deleted_at IS NULL AND e.status = 'approved' AND e.category_id IS NOT NULL;"
        ))
        .map_err(err)?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(err)?;
    rows.collect::<rusqlite::Result<_>>().map_err(err)
}

/// When a model of this kind was last trained or fitted, kept or not.
pub fn last_trained_at(conn: &Connection, kind: &str) -> Result<Option<u64>> {
    conn.query_row(
        "SELECT MAX(trained_at) FROM model_artifacts WHERE kind = ?1;",
        params![kind],
        |row| row.get::<_, Option<i64>>(0),
    )
    .map(|at| at.map(|at| at as u64))
    .map_err(err)
}

/// Verdicts recorded after `after` (calibration refit trigger).
pub fn verdicts_since(conn: &Connection, after: u64, now: u64) -> Result<u32> {
    let since = learning_since(conn)?;
    conn.query_row(
        &format!("SELECT COUNT(*) {VERDICTS} AND s.updated_at > ?3;"),
        params![
            since as i64,
            now.saturating_sub(AUTO_TRUSTED_AFTER_MS) as i64,
            after as i64
        ],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as u32)
    .map_err(err)
}

/// Labeled entries approved since `since` (retraining trigger).
pub fn labeled_since(conn: &Connection, since: u64) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM time_entries e JOIN entry_embeddings v ON v.entry_id = e.id
         WHERE e.deleted_at IS NULL AND e.status = 'approved' AND e.category_id IS NOT NULL
           AND e.updated_at >= ?1;",
        params![since as i64],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as u32)
    .map_err(err)
}

/// Maps personal-model labels back to field values (`none` = no project).
pub fn label_from_model(label: &str) -> Label {
    (label != "none").then(|| label.to_string())
}
