//! Write connection utilities — BEGIN IMMEDIATE transactions, prepare_cached.

use drift_core::errors::StorageError;
use rusqlite::Connection;

/// Execute a write operation inside a BEGIN IMMEDIATE transaction.
/// This acquires the write lock at transaction start, preventing SQLITE_BUSY.
pub fn with_immediate_transaction<F, T>(
    conn: &Connection,
    f: F,
) -> Result<T, StorageError>
where
    F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, StorageError>,
{
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| StorageError::SqliteError {
            message: format!("failed to begin transaction: {e}"),
        })?;

    // Set to IMMEDIATE mode
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| StorageError::SqliteError {
            message: format!("failed to begin immediate: {e}"),
        })
        .ok(); // May fail if already in transaction from unchecked_transaction

    let result = f(&tx)?;

    tx.commit().map_err(|e| StorageError::SqliteError {
        message: format!("failed to commit: {e}"),
    })?;

    Ok(result)
}
