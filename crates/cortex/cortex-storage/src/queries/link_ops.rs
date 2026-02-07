//! Pattern/constraint/file/function link CRUD.

use rusqlite::{params, Connection};

use cortex_core::errors::CortexResult;
use cortex_core::memory::{ConstraintLink, FileLink, FunctionLink, PatternLink};

use crate::to_storage_err;

pub fn add_pattern_link(
    conn: &Connection,
    memory_id: &str,
    link: &PatternLink,
) -> CortexResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO memory_patterns (memory_id, pattern_id, pattern_name)
         VALUES (?1, ?2, ?3)",
        params![memory_id, link.pattern_id, link.pattern_name],
    )
    .map_err(|e| to_storage_err(e.to_string()))?;

    let delta = serde_json::json!({ "link_type": "pattern", "target": link.pattern_id });
    let _ = crate::temporal_events::emit_event(
        conn,
        memory_id,
        "link_added",
        &delta,
        "system",
        "link_ops",
    );
    Ok(())
}

pub fn add_constraint_link(
    conn: &Connection,
    memory_id: &str,
    link: &ConstraintLink,
) -> CortexResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO memory_constraints (memory_id, constraint_id, constraint_name)
         VALUES (?1, ?2, ?3)",
        params![memory_id, link.constraint_id, link.constraint_name],
    )
    .map_err(|e| to_storage_err(e.to_string()))?;

    let delta = serde_json::json!({ "link_type": "constraint", "target": link.constraint_id });
    let _ = crate::temporal_events::emit_event(
        conn,
        memory_id,
        "link_added",
        &delta,
        "system",
        "link_ops",
    );
    Ok(())
}

pub fn add_file_link(conn: &Connection, memory_id: &str, link: &FileLink) -> CortexResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO memory_files (memory_id, file_path, line_start, line_end, content_hash)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            memory_id,
            link.file_path,
            link.line_start,
            link.line_end,
            link.content_hash,
        ],
    )
    .map_err(|e| to_storage_err(e.to_string()))?;

    let delta = serde_json::json!({ "link_type": "file", "target": link.file_path });
    let _ = crate::temporal_events::emit_event(
        conn,
        memory_id,
        "link_added",
        &delta,
        "system",
        "link_ops",
    );
    Ok(())
}

pub fn add_function_link(
    conn: &Connection,
    memory_id: &str,
    link: &FunctionLink,
) -> CortexResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO memory_functions (memory_id, function_name, file_path, signature)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            memory_id,
            link.function_name,
            link.file_path,
            link.signature,
        ],
    )
    .map_err(|e| to_storage_err(e.to_string()))?;

    let delta = serde_json::json!({ "link_type": "function", "target": link.function_name });
    let _ = crate::temporal_events::emit_event(
        conn,
        memory_id,
        "link_added",
        &delta,
        "system",
        "link_ops",
    );
    Ok(())
}
