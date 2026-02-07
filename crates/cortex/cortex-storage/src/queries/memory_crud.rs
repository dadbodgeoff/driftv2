//! Insert, update, get, delete, bulk ops for memories.

use rusqlite::{params, Connection};

use cortex_core::errors::CortexResult;
use cortex_core::memory::{BaseMemory, Confidence, Importance, MemoryType, TypedContent};

use crate::to_storage_err;

/// Insert a single memory into the database.
pub fn insert_memory(conn: &Connection, memory: &BaseMemory) -> CortexResult<()> {
    let content_json =
        serde_json::to_string(&memory.content).map_err(|e| to_storage_err(e.to_string()))?;
    let tags_json =
        serde_json::to_string(&memory.tags).map_err(|e| to_storage_err(e.to_string()))?;
    let memory_type_str =
        serde_json::to_string(&memory.memory_type).map_err(|e| to_storage_err(e.to_string()))?;
    let importance_str =
        serde_json::to_string(&memory.importance).map_err(|e| to_storage_err(e.to_string()))?;

    conn.execute(
        "INSERT INTO memories (
            id, memory_type, content, summary, transaction_time, valid_time,
            valid_until, confidence, importance, last_accessed, access_count,
            tags, archived, superseded_by, supersedes, content_hash
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
        )",
        params![
            memory.id,
            memory_type_str.trim_matches('"'),
            content_json,
            memory.summary,
            memory.transaction_time.to_rfc3339(),
            memory.valid_time.to_rfc3339(),
            memory.valid_until.map(|t| t.to_rfc3339()),
            memory.confidence.value(),
            importance_str.trim_matches('"'),
            memory.last_accessed.to_rfc3339(),
            memory.access_count,
            tags_json,
            memory.archived as i32,
            memory.superseded_by,
            memory.supersedes,
            memory.content_hash,
        ],
    )
    .map_err(|e| to_storage_err(e.to_string()))?;

    // Insert links into their respective tables.
    insert_links(conn, memory)?;

    // Emit Created event (CR3: same transaction).
    let delta = serde_json::to_value(memory).unwrap_or_default();
    let _ = crate::temporal_events::emit_event(
        conn,
        &memory.id,
        "created",
        &delta,
        "system",
        "memory_crud",
    );

    Ok(())
}

/// Insert all link types for a memory.
fn insert_links(conn: &Connection, memory: &BaseMemory) -> CortexResult<()> {
    for link in &memory.linked_patterns {
        conn.execute(
            "INSERT OR IGNORE INTO memory_patterns (memory_id, pattern_id, pattern_name) VALUES (?1, ?2, ?3)",
            params![memory.id, link.pattern_id, link.pattern_name],
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    }
    for link in &memory.linked_constraints {
        conn.execute(
            "INSERT OR IGNORE INTO memory_constraints (memory_id, constraint_id, constraint_name) VALUES (?1, ?2, ?3)",
            params![memory.id, link.constraint_id, link.constraint_name],
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    }
    for link in &memory.linked_files {
        conn.execute(
            "INSERT OR IGNORE INTO memory_files (memory_id, file_path, line_start, line_end, content_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![memory.id, link.file_path, link.line_start, link.line_end, link.content_hash],
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    }
    for link in &memory.linked_functions {
        conn.execute(
            "INSERT OR IGNORE INTO memory_functions (memory_id, function_name, file_path, signature) VALUES (?1, ?2, ?3, ?4)",
            params![memory.id, link.function_name, link.file_path, link.signature],
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    }
    Ok(())
}

/// Get a single memory by ID, including all links.
pub fn get_memory(conn: &Connection, id: &str) -> CortexResult<Option<BaseMemory>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, memory_type, content, summary, transaction_time, valid_time,
                    valid_until, confidence, importance, last_accessed, access_count,
                    tags, archived, superseded_by, supersedes, content_hash
             FROM memories WHERE id = ?1",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;

    let result = stmt
        .query_row(params![id], |row| Ok(row_to_base_memory(row)))
        .optional()
        .map_err(|e| to_storage_err(e.to_string()))?;

    match result {
        Some(Ok(mut memory)) => {
            load_links(conn, &mut memory)?;
            Ok(Some(memory))
        }
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

/// Update an existing memory.
pub fn update_memory(conn: &Connection, memory: &BaseMemory) -> CortexResult<()> {
    // Fetch old state for diff-based event emission.
    let old = get_memory(conn, &memory.id)?;

    let content_json =
        serde_json::to_string(&memory.content).map_err(|e| to_storage_err(e.to_string()))?;
    let tags_json =
        serde_json::to_string(&memory.tags).map_err(|e| to_storage_err(e.to_string()))?;
    let memory_type_str =
        serde_json::to_string(&memory.memory_type).map_err(|e| to_storage_err(e.to_string()))?;
    let importance_str =
        serde_json::to_string(&memory.importance).map_err(|e| to_storage_err(e.to_string()))?;

    let rows = conn
        .execute(
            "UPDATE memories SET
                memory_type = ?2, content = ?3, summary = ?4,
                transaction_time = ?5, valid_time = ?6, valid_until = ?7,
                confidence = ?8, importance = ?9, last_accessed = ?10,
                access_count = ?11, tags = ?12, archived = ?13,
                superseded_by = ?14, supersedes = ?15, content_hash = ?16,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![
                memory.id,
                memory_type_str.trim_matches('"'),
                content_json,
                memory.summary,
                memory.transaction_time.to_rfc3339(),
                memory.valid_time.to_rfc3339(),
                memory.valid_until.map(|t| t.to_rfc3339()),
                memory.confidence.value(),
                importance_str.trim_matches('"'),
                memory.last_accessed.to_rfc3339(),
                memory.access_count,
                tags_json,
                memory.archived as i32,
                memory.superseded_by,
                memory.supersedes,
                memory.content_hash,
            ],
        )
        .map_err(|e| to_storage_err(e.to_string()))?;

    if rows == 0 {
        return Err(cortex_core::CortexError::MemoryNotFound {
            id: memory.id.clone(),
        });
    }

    // Re-insert links (delete old, insert new).
    delete_links(conn, &memory.id)?;
    insert_links(conn, memory)?;

    // Emit events based on changed fields (CR3: same transaction).
    if let Some(ref old_mem) = old {
        if old_mem.content_hash != memory.content_hash {
            let delta = serde_json::json!({
                "old_summary": old_mem.summary,
                "new_summary": memory.summary,
                "old_content_hash": old_mem.content_hash,
                "new_content_hash": memory.content_hash,
            });
            let _ = crate::temporal_events::emit_event(
                conn,
                &memory.id,
                "content_updated",
                &delta,
                "system",
                "memory_crud",
            );
        }
        if old_mem.tags != memory.tags {
            let added: Vec<_> = memory
                .tags
                .iter()
                .filter(|t| !old_mem.tags.contains(t))
                .collect();
            let removed: Vec<_> = old_mem
                .tags
                .iter()
                .filter(|t| !memory.tags.contains(t))
                .collect();
            let delta = serde_json::json!({ "added": added, "removed": removed });
            let _ = crate::temporal_events::emit_event(
                conn,
                &memory.id,
                "tags_modified",
                &delta,
                "system",
                "memory_crud",
            );
        }
        if (old_mem.confidence.value() - memory.confidence.value()).abs() > f64::EPSILON {
            let delta = serde_json::json!({
                "old": old_mem.confidence.value(),
                "new": memory.confidence.value(),
                "reason": "update",
            });
            let _ = crate::temporal_events::emit_event(
                conn,
                &memory.id,
                "confidence_changed",
                &delta,
                "system",
                "memory_crud",
            );
        }
        if old_mem.importance != memory.importance {
            let delta = serde_json::json!({
                "old": serde_json::to_string(&old_mem.importance).unwrap_or_default().trim_matches('"'),
                "new": serde_json::to_string(&memory.importance).unwrap_or_default().trim_matches('"'),
                "reason": "update",
            });
            let _ = crate::temporal_events::emit_event(
                conn,
                &memory.id,
                "importance_changed",
                &delta,
                "system",
                "memory_crud",
            );
        }
        if old_mem.archived != memory.archived {
            if memory.archived {
                let delta = serde_json::json!({ "reason": "update" });
                let _ = crate::temporal_events::emit_event(
                    conn,
                    &memory.id,
                    "archived",
                    &delta,
                    "system",
                    "memory_crud",
                );
            } else {
                let delta = serde_json::json!({});
                let _ = crate::temporal_events::emit_event(
                    conn,
                    &memory.id,
                    "restored",
                    &delta,
                    "system",
                    "memory_crud",
                );
            }
        }
    }

    Ok(())
}

/// Delete a memory by ID.
pub fn delete_memory(conn: &Connection, id: &str) -> CortexResult<()> {
    // Emit Archived event BEFORE the row is deleted (hard DELETE).
    let delta = serde_json::json!({ "reason": "hard_delete" });
    let _ =
        crate::temporal_events::emit_event(conn, id, "archived", &delta, "system", "memory_crud");

    delete_links(conn, id)?;
    conn.execute("DELETE FROM memories WHERE id = ?1", params![id])
        .map_err(|e| to_storage_err(e.to_string()))?;
    Ok(())
}

/// Bulk insert memories. Returns the number successfully inserted.
pub fn bulk_insert(conn: &Connection, memories: &[BaseMemory]) -> CortexResult<usize> {
    let mut count = 0;
    for memory in memories {
        insert_memory(conn, memory)?;
        count += 1;
    }
    Ok(count)
}

/// Bulk get memories by IDs.
pub fn bulk_get(conn: &Connection, ids: &[String]) -> CortexResult<Vec<BaseMemory>> {
    let mut results = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(memory) = get_memory(conn, id)? {
            results.push(memory);
        }
    }
    Ok(results)
}

/// Delete all links for a memory.
fn delete_links(conn: &Connection, memory_id: &str) -> CortexResult<()> {
    for table in &[
        "memory_patterns",
        "memory_constraints",
        "memory_files",
        "memory_functions",
    ] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE memory_id = ?1"),
            params![memory_id],
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    }
    Ok(())
}

/// Load all links for a memory from the link tables.
fn load_links(conn: &Connection, memory: &mut BaseMemory) -> CortexResult<()> {
    // Patterns
    let mut stmt = conn
        .prepare("SELECT pattern_id, pattern_name FROM memory_patterns WHERE memory_id = ?1")
        .map_err(|e| to_storage_err(e.to_string()))?;
    memory.linked_patterns = stmt
        .query_map(params![memory.id], |row| {
            Ok(cortex_core::memory::PatternLink {
                pattern_id: row.get(0)?,
                pattern_name: row.get(1)?,
            })
        })
        .map_err(|e| to_storage_err(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| to_storage_err(e.to_string()))?;

    // Constraints
    let mut stmt = conn
        .prepare(
            "SELECT constraint_id, constraint_name FROM memory_constraints WHERE memory_id = ?1",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    memory.linked_constraints = stmt
        .query_map(params![memory.id], |row| {
            Ok(cortex_core::memory::ConstraintLink {
                constraint_id: row.get(0)?,
                constraint_name: row.get(1)?,
            })
        })
        .map_err(|e| to_storage_err(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| to_storage_err(e.to_string()))?;

    // Files
    let mut stmt = conn
        .prepare(
            "SELECT file_path, line_start, line_end, content_hash FROM memory_files WHERE memory_id = ?1",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    memory.linked_files = stmt
        .query_map(params![memory.id], |row| {
            Ok(cortex_core::memory::FileLink {
                file_path: row.get(0)?,
                line_start: row.get(1)?,
                line_end: row.get(2)?,
                content_hash: row.get(3)?,
            })
        })
        .map_err(|e| to_storage_err(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| to_storage_err(e.to_string()))?;

    // Functions
    let mut stmt = conn
        .prepare(
            "SELECT function_name, file_path, signature FROM memory_functions WHERE memory_id = ?1",
        )
        .map_err(|e| to_storage_err(e.to_string()))?;
    memory.linked_functions = stmt
        .query_map(params![memory.id], |row| {
            Ok(cortex_core::memory::FunctionLink {
                function_name: row.get(0)?,
                file_path: row.get(1)?,
                signature: row.get(2)?,
            })
        })
        .map_err(|e| to_storage_err(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| to_storage_err(e.to_string()))?;

    Ok(())
}

/// Parse a row from the memories table into a BaseMemory.
pub(crate) fn row_to_base_memory(row: &rusqlite::Row<'_>) -> CortexResult<BaseMemory> {
    let memory_type_str: String = row.get(1).map_err(|e| to_storage_err(e.to_string()))?;
    let content_json: String = row.get(2).map_err(|e| to_storage_err(e.to_string()))?;
    let tags_json: String = row.get(11).map_err(|e| to_storage_err(e.to_string()))?;
    let importance_str: String = row.get(8).map_err(|e| to_storage_err(e.to_string()))?;
    let valid_until_str: Option<String> = row.get(6).map_err(|e| to_storage_err(e.to_string()))?;

    let memory_type: MemoryType = serde_json::from_str(&format!("\"{memory_type_str}\""))
        .map_err(|e| to_storage_err(format!("parse memory_type '{memory_type_str}': {e}")))?;
    let content: TypedContent = serde_json::from_str(&content_json)
        .map_err(|e| to_storage_err(format!("parse content: {e}")))?;
    let tags: Vec<String> =
        serde_json::from_str(&tags_json).map_err(|e| to_storage_err(format!("parse tags: {e}")))?;
    let importance: Importance = serde_json::from_str(&format!("\"{importance_str}\""))
        .map_err(|e| to_storage_err(format!("parse importance '{importance_str}': {e}")))?;

    let tx_time_str: String = row.get(4).map_err(|e| to_storage_err(e.to_string()))?;
    let valid_time_str: String = row.get(5).map_err(|e| to_storage_err(e.to_string()))?;
    let last_accessed_str: String = row.get(9).map_err(|e| to_storage_err(e.to_string()))?;

    let parse_dt = |s: &str| -> CortexResult<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| to_storage_err(format!("parse datetime '{s}': {e}")))
    };

    Ok(BaseMemory {
        id: row.get(0).map_err(|e| to_storage_err(e.to_string()))?,
        memory_type,
        content,
        summary: row.get(3).map_err(|e| to_storage_err(e.to_string()))?,
        transaction_time: parse_dt(&tx_time_str)?,
        valid_time: parse_dt(&valid_time_str)?,
        valid_until: valid_until_str.as_deref().map(parse_dt).transpose()?,
        confidence: Confidence::new(row.get(7).map_err(|e| to_storage_err(e.to_string()))?),
        importance,
        last_accessed: parse_dt(&last_accessed_str)?,
        access_count: row
            .get::<_, i64>(10)
            .map_err(|e| to_storage_err(e.to_string()))? as u64,
        linked_patterns: Vec::new(),
        linked_constraints: Vec::new(),
        linked_files: Vec::new(),
        linked_functions: Vec::new(),
        tags,
        archived: row
            .get::<_, i32>(12)
            .map_err(|e| to_storage_err(e.to_string()))?
            != 0,
        superseded_by: row.get(13).map_err(|e| to_storage_err(e.to_string()))?,
        supersedes: row.get(14).map_err(|e| to_storage_err(e.to_string()))?,
        content_hash: row.get(15).map_err(|e| to_storage_err(e.to_string()))?,
    })
}

/// Helper trait to make `query_row` return `Option` on not-found.
trait OptionalRow<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalRow<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Re-export for use in other query modules.
pub(crate) use self::row_to_base_memory as parse_memory_row;
