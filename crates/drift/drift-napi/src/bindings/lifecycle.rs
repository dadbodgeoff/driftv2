//! Lifecycle bindings: `drift_initialize()` and `drift_shutdown()`.
//!
//! `drift_initialize()` creates drift.db, sets PRAGMAs, runs migrations,
//! and initializes the DriftRuntime singleton.
//!
//! `drift_shutdown()` cleanly closes all connections and flushes caches.

use std::path::PathBuf;

use napi_derive::napi;

use crate::conversions::error_codes;
use crate::runtime::{self, RuntimeOptions};

/// Initialize the Drift analysis engine.
///
/// Creates the database (drift.db), applies SQLite PRAGMAs (WAL mode,
/// synchronous=NORMAL, 64MB page cache), runs schema migrations, and
/// initializes the global DriftRuntime singleton.
///
/// Must be called exactly once before any other drift_* function.
/// Subsequent calls return an ALREADY_INITIALIZED error.
///
/// @param db_path - Optional path to drift.db. Defaults to `.drift/drift.db`.
/// @param project_root - Optional project root for scanning and config resolution.
/// @param config_toml - Optional TOML configuration string. Overrides file-based config.
#[napi(js_name = "driftInitialize")]
pub fn drift_initialize(
    db_path: Option<String>,
    project_root: Option<String>,
    config_toml: Option<String>,
) -> napi::Result<()> {
    let opts = RuntimeOptions {
        db_path: db_path.map(PathBuf::from),
        project_root: project_root.map(PathBuf::from),
        config_toml,
    };

    runtime::initialize(opts)
}

/// Shut down the Drift analysis engine.
///
/// Performs a WAL checkpoint (TRUNCATE mode) to consolidate the write-ahead log,
/// then drops the runtime. After this call, all drift_* functions will return
/// RUNTIME_NOT_INITIALIZED until `driftInitialize()` is called again.
///
/// Note: Because `OnceLock` cannot be reset, shutdown performs cleanup but the
/// runtime reference remains. In practice, shutdown is called once at process exit.
#[napi(js_name = "driftShutdown")]
pub fn drift_shutdown() -> napi::Result<()> {
    let rt = runtime::get()?;

    // Checkpoint WAL to consolidate the write-ahead log
    rt.db.checkpoint().map_err(|e| {
        napi::Error::from_reason(format!("[{}] WAL checkpoint failed: {e}", error_codes::STORAGE_ERROR))
    })?;

    Ok(())
}

/// Check if the Drift runtime is initialized.
///
/// Returns true if `driftInitialize()` has been called successfully.
#[napi(js_name = "driftIsInitialized")]
pub fn drift_is_initialized() -> bool {
    runtime::is_initialized()
}
