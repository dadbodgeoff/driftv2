//! CortexRuntime — owns all engines, background task scheduler, graceful shutdown.
//!
//! The runtime is a singleton stored behind `OnceLock`. It's initialized once
//! via `initialize()` and accessed via `get()` for the lifetime of the process.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use cortex_causal::CausalEngine;
use cortex_cloud::CloudEngine;
use cortex_compression::CompressionEngine;
use cortex_consolidation::ConsolidationEngine;
use cortex_core::config::CortexConfig;
use cortex_core::errors::CortexResult;
use cortex_decay::DecayEngine;
use cortex_embeddings::EmbeddingEngine;
use cortex_learning::LearningEngine;
use cortex_observability::ObservabilityEngine;
use cortex_prediction::PredictionEngine;
use cortex_privacy::PrivacyEngine;
use cortex_session::SessionManager;
use cortex_storage::StorageEngine;
use cortex_temporal::TemporalEngine;
use cortex_validation::ValidationEngine;

use crate::conversions::error_types;

/// Global singleton.
static RUNTIME: OnceLock<Arc<CortexRuntime>> = OnceLock::new();

/// The central runtime owning all Cortex engines.
///
/// Engines that require `&mut self` are wrapped in `Mutex` to allow
/// safe concurrent access from async NAPI callbacks.
pub struct CortexRuntime {
    pub storage: StorageEngine,
    pub embeddings: Mutex<EmbeddingEngine>,
    pub compression: CompressionEngine,
    pub causal: CausalEngine,
    pub decay: DecayEngine,
    pub validation: ValidationEngine,
    pub learning: Mutex<LearningEngine>,
    pub consolidation: Mutex<ConsolidationEngine>,
    pub prediction: PredictionEngine<StorageEngine>,
    pub session: SessionManager,
    pub privacy: PrivacyEngine,
    pub observability: Mutex<ObservabilityEngine>,
    pub cloud: Option<Mutex<CloudEngine>>,
    pub temporal: TemporalEngine,
    pub config: CortexConfig,
}

/// Options for initializing the runtime.
#[derive(Default)]
pub struct RuntimeOptions {
    /// Path to the SQLite database. If None, uses in-memory.
    pub db_path: Option<PathBuf>,
    /// TOML configuration string. If None, uses defaults.
    pub config_toml: Option<String>,
    /// Whether to enable cloud sync.
    pub cloud_enabled: bool,
}

impl CortexRuntime {
    /// Create a new runtime with the given options.
    fn new(opts: RuntimeOptions) -> CortexResult<Self> {
        let config = match &opts.config_toml {
            Some(toml_str) => CortexConfig::from_toml(toml_str)
                .map_err(|e| cortex_core::CortexError::ConfigError(e.to_string()))?,
            None => CortexConfig::default(),
        };

        // Storage
        let storage = match &opts.db_path {
            Some(path) => StorageEngine::open(path)?,
            None => StorageEngine::open_in_memory()?,
        };

        // Embeddings
        let embeddings = EmbeddingEngine::new(config.embedding.clone());

        // Compression
        let compression = CompressionEngine::new();

        // Causal
        let causal = CausalEngine::new();

        // Decay
        let decay = DecayEngine::new();

        // Validation
        let validation = ValidationEngine::default();

        // Learning
        let learning = LearningEngine::new();

        // Consolidation — needs an embedding provider
        let consolidation_embedder = EmbeddingEngine::new(config.embedding.clone());
        let consolidation = ConsolidationEngine::new(Box::new(consolidation_embedder));

        // Prediction — needs storage (clone not available, so open a second handle)
        let prediction_storage = match &opts.db_path {
            Some(path) => StorageEngine::open(path)?,
            None => StorageEngine::open_in_memory()?,
        };
        let prediction = PredictionEngine::new(prediction_storage);

        // Session
        let session = SessionManager::new();

        // Privacy
        let privacy = PrivacyEngine::new();

        // Observability
        let observability = ObservabilityEngine::new();

        // Cloud (optional)
        let cloud = if opts.cloud_enabled {
            Some(Mutex::new(CloudEngine::new(
                cortex_cloud::auth::login_flow::AuthMethod::ApiKey(String::new()),
                cortex_cloud::HttpClientConfig::default(),
                cortex_cloud::QuotaLimits::default(),
            )))
        } else {
            None
        };

        // Temporal — opens its own connections to the same database
        let temporal = {
            let (writer, readers) = match &opts.db_path {
                Some(path) => {
                    let w = cortex_storage::pool::WriteConnection::open(path)?;
                    let r = cortex_storage::pool::ReadPool::open(path, 4)?;
                    (w, r)
                }
                None => {
                    let w = cortex_storage::pool::WriteConnection::open_in_memory()?;
                    let r = cortex_storage::pool::ReadPool::open_in_memory(4)?;
                    (w, r)
                }
            };
            TemporalEngine::new(
                Arc::new(writer),
                Arc::new(readers),
                config.temporal.clone(),
            )
        };

        Ok(Self {
            storage,
            embeddings: Mutex::new(embeddings),
            compression,
            causal,
            decay,
            validation,
            learning: Mutex::new(learning),
            consolidation: Mutex::new(consolidation),
            prediction,
            session,
            privacy,
            observability: Mutex::new(observability),
            cloud,
            temporal,
            config,
        })
    }
}

/// Initialize the global CortexRuntime singleton.
///
/// Returns an error if already initialized or if initialization fails.
pub fn initialize(opts: RuntimeOptions) -> napi::Result<()> {
    let runtime = CortexRuntime::new(opts).map_err(error_types::to_napi_error)?;
    RUNTIME
        .set(Arc::new(runtime))
        .map_err(|_| napi::Error::from_reason("CortexRuntime already initialized"))
}

/// Get a reference to the global CortexRuntime.
///
/// Returns an error if not yet initialized.
pub fn get() -> napi::Result<Arc<CortexRuntime>> {
    RUNTIME
        .get()
        .cloned()
        .ok_or_else(error_types::runtime_not_initialized)
}

/// Check if the runtime has been initialized.
pub fn is_initialized() -> bool {
    RUNTIME.get().is_some()
}
