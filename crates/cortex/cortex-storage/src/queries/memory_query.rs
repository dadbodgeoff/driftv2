//! Query memories by type, importance, confidence range, date range, tags.

use rusqlite::{params, Connection};

use chrono::{DateTime, Utc};
use cortex_core::errors::CortexResult;
use cortex_core::memory::{BaseMemory, Importance, MemoryType};

use super::memory_crud::parse_memory_row;
use crate::to_storage_err;

/// Query memories by type.
pub fn query_by_type(conn: &Connection, memory_type: MemoryType) -> CortexResult<Vec<BaseMemory>> {
    let type_str =
        serde_json::to_string(&memory_type).map_err(|e| to_storage_err(e.to_string()))?;
    let type_str = type_str.trim_matches('"');

    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, summary, transaction_time, valid_time,
                    valid_until, confidence, importance, last_accessed, access_count,
                    tags, archived, superseded_by, supersedes, content_hash
             FROM memories WHERE memory_type = ?1 AND archived = 0",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;

    collect_memories(&mut stmt, params![type_str])
}

/// Query memories with importance >= min.
pub fn query_by_importance(conn: &Connection, min: Importance) -> CortexResult<Vec<BaseMemory>> {
    let importance_values = match min {
        Importance::Low => vec!["low", "normal", "high", "critical"],
        Importance::Normal => vec!["normal", "high", "critical"],
        Importance::High => vec!["high", "critical"],
        Importance::Critical => vec!["critical"],
    };

    let placeholders: Vec<String> = importance_values
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect();
    let sql = format!(
        "SELECT id, memory_type, content, summary, transaction_time, valid_time,
                valid_until, confidence, importance, last_accessed, access_count,
                tags, archived, superseded_by, supersedes, content_hash
         FROM memories WHERE importance IN ({}) AND archived = 0",
        placeholders.join(", ")
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| to_storage_err(e.to_string()))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = importance_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt
        .query_map(params.as_slice(), |row| Ok(parse_memory_row(row)))
        .map_err(|e| to_storage_err(e.to_string()))?;

    let mut results = Vec::new();
    for row in rows {
        let memory = row.map_err(|e| to_storage_err(e.to_string()))??;
        results.push(memory);
    }
    Ok(results)
}

/// Query memories within a confidence range.
pub fn query_by_confidence_range(
    conn: &Connection,
    min: f64,
    max: f64,
) -> CortexResult<Vec<BaseMemory>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, summary, transaction_time, valid_time,
                    valid_until, confidence, importance, last_accessed, access_count,
                    tags, archived, superseded_by, supersedes, content_hash
             FROM memories WHERE confidence >= ?1 AND confidence <= ?2 AND archived = 0",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;

    collect_memories(&mut stmt, params![min, max])
}

/// Query memories within a date range (by transaction_time).
pub fn query_by_date_range(
    conn: &Connection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> CortexResult<Vec<BaseMemory>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, summary, transaction_time, valid_time,
                    valid_until, confidence, importance, last_accessed, access_count,
                    tags, archived, superseded_by, supersedes, content_hash
             FROM memories WHERE transaction_time >= ?1 AND transaction_time <= ?2 AND archived = 0",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;

    collect_memories(&mut stmt, params![from.to_rfc3339(), to.to_rfc3339()])
}

/// Query memories by tags (any match).
pub fn query_by_tags(conn: &Connection, tags: &[String]) -> CortexResult<Vec<BaseMemory>> {
    // Use JSON array contains for tag matching.
    let mut results = Vec::new();
    for tag in tags {
        let pattern = format!("%\"{tag}\"%");
        let mut stmt = conn
            .prepare(
                "SELECT id, memory_type, content, summary, transaction_time, valid_time,
                        valid_until, confidence, importance, last_accessed, access_count,
                        tags, archived, superseded_by, supersedes, content_hash
                 FROM memories WHERE tags LIKE ?1 AND archived = 0",
            )
            .map_err(|e| to_storage_err(e.to_string()))?;

        let rows = stmt
            .query_map(params![pattern], |row| Ok(parse_memory_row(row)))
            .map_err(|e| to_storage_err(e.to_string()))?;

        for row in rows {
            let memory = row.map_err(|e| to_storage_err(e.to_string()))??;
            if !results.iter().any(|m: &BaseMemory| m.id == memory.id) {
                results.push(memory);
            }
        }
    }
    Ok(results)
}

/// Helper: collect memories from a prepared statement.
fn collect_memories(
    stmt: &mut rusqlite::Statement<'_>,
    params: impl rusqlite::Params,
) -> CortexResult<Vec<BaseMemory>> {
    let rows = stmt
        .query_map(params, |row| Ok(parse_memory_row(row)))
        .map_err(|e| to_storage_err(e.to_string()))?;

    let mut results = Vec::new();
    for row in rows {
        let memory = row.map_err(|e| to_storage_err(e.to_string()))??;
        results.push(memory);
    }
    Ok(results)
}
