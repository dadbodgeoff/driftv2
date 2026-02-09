//! Dedicated writer thread with crossbeam-channel bounded(1024).
//! Batches writes into single transactions for throughput.

use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use drift_core::errors::StorageError;
use rusqlite::Connection;

use super::commands::{
    BatchCommand, CallEdgeRow, BoundaryRow, ConventionInsertRow, DetectionRow,
    FileMetadataRow, FunctionRow, OutlierDetectionRow, ParseCacheRow, PatternConfidenceRow,
};

const CHANNEL_BOUND: usize = 1024;
const BATCH_SIZE: usize = 500;
const FLUSH_TIMEOUT: Duration = Duration::from_millis(100);

/// Statistics from the batch writer.
#[derive(Debug, Default, Clone)]
pub struct WriteStats {
    pub file_metadata_rows: usize,
    pub parse_cache_rows: usize,
    pub function_rows: usize,
    pub deleted_files: usize,
    pub call_edge_rows: usize,
    pub detection_rows: usize,
    pub boundary_rows: usize,
    pub flushes: usize,
}

/// A batch writer that accepts commands via a channel and writes them
/// in batched transactions on a dedicated thread.
pub struct BatchWriter {
    tx: Sender<BatchCommand>,
    handle: Option<JoinHandle<Result<WriteStats, StorageError>>>,
}

impl BatchWriter {
    /// Create a new batch writer with a dedicated writer thread.
    /// The `conn` is moved to the writer thread.
    pub fn new(conn: Connection) -> Self {
        let (tx, rx) = bounded(CHANNEL_BOUND);

        let handle = thread::Builder::new()
            .name("drift-batch-writer".to_string())
            .spawn(move || writer_loop(conn, rx))
            .expect("failed to spawn batch writer thread");

        Self {
            tx,
            handle: Some(handle),
        }
    }

    /// Send a command to the batch writer.
    pub fn send(&self, cmd: BatchCommand) -> Result<(), StorageError> {
        self.tx.send(cmd).map_err(|_| StorageError::SqliteError {
            message: "batch writer channel disconnected".to_string(),
        })
    }

    /// Flush pending writes.
    pub fn flush(&self) -> Result<(), StorageError> {
        self.send(BatchCommand::Flush)
    }

    /// Shut down the writer thread and wait for completion.
    pub fn shutdown(mut self) -> Result<WriteStats, StorageError> {
        let _ = self.tx.send(BatchCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| StorageError::SqliteError {
                message: "batch writer thread panicked".to_string(),
            })?
        } else {
            Ok(WriteStats::default())
        }
    }
}

impl Drop for BatchWriter {
    fn drop(&mut self) {
        // Signal shutdown if not already done
        let _ = self.tx.send(BatchCommand::Shutdown);
    }
}

fn writer_loop(
    conn: Connection,
    rx: Receiver<BatchCommand>,
) -> Result<WriteStats, StorageError> {
    let mut buffer: Vec<BatchCommand> = Vec::with_capacity(BATCH_SIZE);
    let mut stats = WriteStats::default();

    loop {
        match rx.recv_timeout(FLUSH_TIMEOUT) {
            Ok(BatchCommand::Shutdown) => {
                flush_buffer(&conn, &mut buffer, &mut stats)?;
                break;
            }
            Ok(BatchCommand::Flush) => {
                flush_buffer(&conn, &mut buffer, &mut stats)?;
            }
            Ok(cmd) => {
                buffer.push(cmd);
                if buffer.len() >= BATCH_SIZE {
                    flush_buffer(&conn, &mut buffer, &mut stats)?;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if !buffer.is_empty() {
                    flush_buffer(&conn, &mut buffer, &mut stats)?;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                flush_buffer(&conn, &mut buffer, &mut stats)?;
                break;
            }
        }
    }

    Ok(stats)
}

fn flush_buffer(
    conn: &Connection,
    buffer: &mut Vec<BatchCommand>,
    stats: &mut WriteStats,
) -> Result<(), StorageError> {
    if buffer.is_empty() {
        return Ok(());
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| StorageError::SqliteError {
            message: format!("begin transaction: {e}"),
        })?;

    for cmd in buffer.drain(..) {
        match cmd {
            BatchCommand::UpsertFileMetadata(rows) => {
                upsert_file_metadata(&tx, &rows)?;
                stats.file_metadata_rows += rows.len();
            }
            BatchCommand::InsertParseCache(rows) => {
                insert_parse_cache(&tx, &rows)?;
                stats.parse_cache_rows += rows.len();
            }
            BatchCommand::InsertFunctions(rows) => {
                insert_functions(&tx, &rows)?;
                stats.function_rows += rows.len();
            }
            BatchCommand::DeleteFileMetadata(paths) => {
                delete_file_metadata(&tx, &paths)?;
                stats.deleted_files += paths.len();
            }
            BatchCommand::InsertCallEdges(rows) => {
                insert_call_edges(&tx, &rows)?;
                stats.call_edge_rows += rows.len();
            }
            BatchCommand::InsertDetections(rows) => {
                insert_detections(&tx, &rows)?;
                stats.detection_rows += rows.len();
            }
            BatchCommand::InsertBoundaries(rows) => {
                insert_boundaries(&tx, &rows)?;
                stats.boundary_rows += rows.len();
            }
            BatchCommand::InsertPatternConfidence(rows) => {
                insert_pattern_confidence(&tx, &rows)?;
            }
            BatchCommand::InsertOutliers(rows) => {
                insert_outlier_rows(&tx, &rows)?;
            }
            BatchCommand::InsertConventions(rows) => {
                insert_convention_rows(&tx, &rows)?;
            }
            BatchCommand::Flush | BatchCommand::Shutdown => {}
        }
    }

    tx.commit().map_err(|e| StorageError::SqliteError {
        message: format!("commit: {e}"),
    })?;
    stats.flushes += 1;

    Ok(())
}

fn upsert_file_metadata(
    conn: &Connection,
    rows: &[FileMetadataRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT OR REPLACE INTO file_metadata
             (path, language, file_size, content_hash, mtime_secs, mtime_nanos,
              last_scanned_at, scan_duration_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.path,
            row.language,
            row.file_size,
            row.content_hash,
            row.mtime_secs,
            row.mtime_nanos,
            row.last_scanned_at,
            row.scan_duration_us,
        ])
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;
    }
    Ok(())
}

fn insert_parse_cache(
    conn: &Connection,
    rows: &[ParseCacheRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT OR REPLACE INTO parse_cache
             (content_hash, language, parse_result_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.content_hash,
            row.language,
            row.parse_result_json,
            row.created_at,
        ])
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;
    }
    Ok(())
}

fn insert_functions(
    conn: &Connection,
    rows: &[FunctionRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT OR REPLACE INTO functions
             (file, name, qualified_name, language, line, end_line,
              parameter_count, return_type, is_exported, is_async,
              body_hash, signature_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.file,
            row.name,
            row.qualified_name,
            row.language,
            row.line,
            row.end_line,
            row.parameter_count,
            row.return_type,
            row.is_exported,
            row.is_async,
            row.body_hash,
            row.signature_hash,
        ])
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;
    }
    Ok(())
}

fn delete_file_metadata(
    conn: &Connection,
    paths: &[String],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached("DELETE FROM file_metadata WHERE path = ?1")
        .map_err(|e| StorageError::SqliteError {
            message: e.to_string(),
        })?;

    for path in paths {
        stmt.execute(rusqlite::params![path])
            .map_err(|e| StorageError::SqliteError {
                message: e.to_string(),
            })?;
    }
    Ok(())
}

fn insert_call_edges(
    conn: &Connection,
    rows: &[CallEdgeRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT OR REPLACE INTO call_edges
             (caller_id, callee_id, resolution, confidence, call_site_line)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.caller_id, row.callee_id, row.resolution,
            row.confidence, row.call_site_line,
        ])
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    }
    Ok(())
}

fn insert_detections(
    conn: &Connection,
    rows: &[DetectionRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT INTO detections
             (file, line, column_num, pattern_id, category, confidence,
              detection_method, cwe_ids, owasp, matched_text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.file, row.line, row.column_num, row.pattern_id,
            row.category, row.confidence, row.detection_method,
            row.cwe_ids, row.owasp, row.matched_text,
        ])
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    }
    Ok(())
}

fn insert_boundaries(
    conn: &Connection,
    rows: &[BoundaryRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT INTO boundaries
             (file, framework, model_name, table_name, field_name, sensitivity, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.file, row.framework, row.model_name, row.table_name,
            row.field_name, row.sensitivity, row.confidence,
        ])
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    }
    Ok(())
}

fn insert_pattern_confidence(
    conn: &Connection,
    rows: &[PatternConfidenceRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT OR REPLACE INTO pattern_confidence
             (pattern_id, alpha, beta, posterior_mean, credible_interval_low,
              credible_interval_high, tier, momentum)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.pattern_id, row.alpha, row.beta, row.posterior_mean,
            row.credible_interval_low, row.credible_interval_high,
            row.tier, row.momentum,
        ])
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    }
    Ok(())
}

fn insert_outlier_rows(
    conn: &Connection,
    rows: &[OutlierDetectionRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT INTO outliers
             (pattern_id, file, line, deviation_score, significance, method)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.pattern_id, row.file, row.line,
            row.deviation_score, row.significance, row.method,
        ])
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    }
    Ok(())
}

fn insert_convention_rows(
    conn: &Connection,
    rows: &[ConventionInsertRow],
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "INSERT INTO conventions
             (pattern_id, category, scope, dominance_ratio, promotion_status,
              discovered_at, last_seen, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    for row in rows {
        stmt.execute(rusqlite::params![
            row.pattern_id, row.category, row.scope, row.dominance_ratio,
            row.promotion_status, row.discovered_at, row.last_seen, row.expires_at,
        ])
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    }
    Ok(())
}
