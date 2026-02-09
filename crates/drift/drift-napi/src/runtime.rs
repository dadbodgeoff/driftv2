//! DriftRuntime — singleton via `OnceLock`, lock-free after initialization.
//!
//! The runtime owns the database manager, configuration, and event dispatcher.
//! It is initialized once via `initialize()` and accessed via `get()` for the
//! lifetime of the process. Scanner/parsers are stateless — no Mutex wrappers needed.
//!
//! Pattern reference: `cortex-napi/src/runtime.rs`

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use drift_core::config::DriftConfig;
use drift_core::events::dispatcher::EventDispatcher;
use drift_core::events::handler::DriftEventHandler;
use drift_storage::DatabaseManager;

use crate::conversions::error_codes;

/// Global singleton — lock-free after first `initialize()` call.
static RUNTIME: OnceLock<Arc<DriftRuntime>> = OnceLock::new();

/// The central runtime owning all Drift subsystems.
///
/// `DatabaseManager` handles its own write serialization internally
/// (`Mutex<Connection>` for writer, read pool for readers).
/// Scanner and parsers are stateless or use `thread_local!` storage,
/// so no additional Mutex wrappers are needed here.
pub struct DriftRuntime {
    pub db: DatabaseManager,
    pub config: DriftConfig,
    pub dispatcher: EventDispatcher,
    pub project_root: Option<PathBuf>,
}

/// Options for initializing the runtime.
#[derive(Default)]
pub struct RuntimeOptions {
    /// Path to drift.db. If None, uses default location (.drift/drift.db).
    pub db_path: Option<PathBuf>,
    /// Path to project root for scanning.
    pub project_root: Option<PathBuf>,
    /// TOML configuration string. If None, uses defaults.
    pub config_toml: Option<String>,
}

impl DriftRuntime {
    /// Create a new runtime with the given options.
    fn new(opts: RuntimeOptions) -> Result<Self, napi::Error> {
        // Resolve configuration
        let config = match &opts.config_toml {
            Some(toml_str) => DriftConfig::from_toml(toml_str).map_err(|e| {
                napi::Error::from_reason(format!("[{}] {e}", error_codes::CONFIG_ERROR))
            })?,
            None => {
                // Try loading from project root, fall back to defaults
                if let Some(ref root) = opts.project_root {
                    DriftConfig::load(root, None).unwrap_or_default()
                } else {
                    DriftConfig::default()
                }
            }
        };

        // Open database
        let db = match &opts.db_path {
            Some(path) => {
                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        napi::Error::from_reason(format!(
                            "[{}] Failed to create database directory: {e}",
                            error_codes::INIT_ERROR
                        ))
                    })?;
                }
                DatabaseManager::open(path).map_err(|e| {
                    napi::Error::from_reason(format!(
                        "[{}] {e}",
                        error_codes::STORAGE_ERROR
                    ))
                })?
            }
            None => {
                // Default: .drift/drift.db relative to project root
                let db_path = opts
                    .project_root
                    .as_deref()
                    .unwrap_or_else(|| Path::new("."))
                    .join(".drift")
                    .join("drift.db");

                if let Some(parent) = db_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        napi::Error::from_reason(format!(
                            "[{}] Failed to create .drift directory: {e}",
                            error_codes::INIT_ERROR
                        ))
                    })?;
                }

                DatabaseManager::open(&db_path).map_err(|e| {
                    napi::Error::from_reason(format!(
                        "[{}] {e}",
                        error_codes::STORAGE_ERROR
                    ))
                })?
            }
        };

        let dispatcher = EventDispatcher::new();

        Ok(Self {
            db,
            config,
            dispatcher,
            project_root: opts.project_root,
        })
    }
}

/// Initialize the global DriftRuntime singleton.
///
/// Returns an error if already initialized or if initialization fails.
/// After this call, `get()` is lock-free.
pub fn initialize(opts: RuntimeOptions) -> napi::Result<()> {
    let runtime = DriftRuntime::new(opts)?;
    RUNTIME.set(Arc::new(runtime)).map_err(|_| {
        napi::Error::from_reason(format!(
            "[{}] DriftRuntime already initialized",
            error_codes::ALREADY_INITIALIZED
        ))
    })
}

/// Get a reference to the global DriftRuntime.
///
/// Returns an error if not yet initialized. Lock-free after init.
pub fn get() -> napi::Result<Arc<DriftRuntime>> {
    RUNTIME.get().cloned().ok_or_else(|| {
        napi::Error::from_reason(format!(
            "[{}] DriftRuntime not initialized. Call driftInitialize() first.",
            error_codes::RUNTIME_NOT_INITIALIZED
        ))
    })
}

/// Check if the runtime has been initialized.
pub fn is_initialized() -> bool {
    RUNTIME.get().is_some()
}
