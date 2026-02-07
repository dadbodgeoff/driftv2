//! Rollback memory to previous version with audit log entry.

use rusqlite::Connection;

use cortex_core::errors::CortexResult;
use cortex_core::models::{AuditActor, AuditOperation};

use crate::audit::AuditLogger;
use crate::queries::version_ops;
use crate::to_storage_err;

/// Rollback a memory to a specific version.
/// Updates the memory's content and creates an audit log entry.
pub fn rollback_to_version(
    conn: &Connection,
    memory_id: &str,
    target_version: i64,
) -> CortexResult<()> {
    let version =
        version_ops::get_at_version(conn, memory_id, target_version)?.ok_or_else(|| {
            to_storage_err(format!(
                "version {target_version} not found for memory {memory_id}"
            ))
        })?;

    // Update the memory's content to the rollback version.
    conn.execute(
        "UPDATE memories SET
            content = ?2,
            summary = ?3,
            confidence = ?4,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1",
        rusqlite::params![
            memory_id,
            version.content,
            version.summary,
            version.confidence,
        ],
    )
    .map_err(|e| to_storage_err(e.to_string()))?;

    // Log the rollback.
    AuditLogger::log(
        conn,
        memory_id,
        AuditOperation::Update,
        AuditActor::System,
        serde_json::json!({
            "action": "rollback",
            "target_version": target_version,
        }),
    )?;

    Ok(())
}
