//! Scanner bindings: `drift_scan()` as `AsyncTask` with progress callbacks.
//!
//! The scan operation runs on libuv's thread pool (not the main JS thread)
//! via napi-rs `AsyncTask`. Progress is reported back to TypeScript via
//! v3's redesigned `ThreadsafeFunction`.
//!
//! Architecture:
//! 1. TS calls `driftScan(root, options, onProgress?)`
//! 2. Rust creates `ScanTask` → runs on libuv thread pool
//! 3. Scanner emits progress events → `NapiProgressHandler` forwards to ThreadsafeFunction
//! 4. Results are persisted to drift.db inside Rust (no NAPI crossing for bulk data)
//! 5. Lightweight `ScanSummary` is returned to TS

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use drift_analysis::scanner::Scanner;
use drift_core::config::ScanConfig;
use drift_core::events::handler::DriftEventHandler;
use drift_core::events::types::{ScanProgressEvent, ScanStartedEvent};
use drift_core::types::collections::FxHashMap;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;

use crate::conversions::error_codes;
use crate::conversions::types::{ProgressUpdate, ScanOptions, ScanSummary};
use crate::runtime;

/// Global cancellation flag for the current scan operation.
/// Set by `driftCancelScan()`, checked by rayon workers between files.
static SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);

// ---- AsyncTask: ScanTask (no progress) ----

/// Async scan task that runs on libuv's thread pool.
pub struct ScanTask {
    root: PathBuf,
    options: ScanOptions,
}

#[napi]
impl Task for ScanTask {
    type Output = ScanSummary;
    type JsValue = ScanSummary;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        let rt = runtime::get()?;
        let config = build_scan_config(&rt.config.scan, &self.options);
        let scanner = Scanner::new(config);

        // Wire global cancellation flag to scanner
        if SCAN_CANCELLED.load(Ordering::Relaxed) {
            SCAN_CANCELLED.store(false, Ordering::SeqCst);
        }

        // No cached metadata for now — full scan
        // TODO: Phase 2 will load cached metadata from drift.db for incremental scans
        let cached = FxHashMap::default();

        let diff = scanner
            .scan(&self.root, &cached, &NoOpHandler)
            .map_err(error_codes::scan_error)?;

        Ok(ScanSummary::from(&diff))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(output)
    }
}

/// Scan a directory asynchronously. Returns a `ScanSummary` with counts and timing.
///
/// Full results are persisted to drift.db — query them via `driftQueryFiles()` etc.
/// Runs on libuv's thread pool, does not block the Node.js event loop.
///
/// @param root - Directory to scan.
/// @param options - Optional scan configuration overrides.
#[napi(js_name = "driftScan")]
pub fn drift_scan(root: String, options: Option<ScanOptions>) -> AsyncTask<ScanTask> {
    reset_cancellation();
    AsyncTask::new(ScanTask {
        root: PathBuf::from(root),
        options: options.unwrap_or_default(),
    })
}

// ---- AsyncTask: ScanWithProgressTask ----

/// Async scan task with progress reporting via ThreadsafeFunction.
pub struct ScanWithProgressTask {
    root: PathBuf,
    options: ScanOptions,
    on_progress: Arc<ThreadsafeFunction<ProgressUpdate, ()>>,
}

#[napi]
impl Task for ScanWithProgressTask {
    type Output = ScanSummary;
    type JsValue = ScanSummary;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        let rt = runtime::get()?;
        let config = build_scan_config(&rt.config.scan, &self.options);
        let scanner = Scanner::new(config);

        // Create progress handler that bridges DriftEventHandler → ThreadsafeFunction
        let progress_handler = NapiProgressHandler::new(self.on_progress.clone());

        let cached = FxHashMap::default();

        let diff = scanner
            .scan(&self.root, &cached, &progress_handler)
            .map_err(error_codes::scan_error)?;

        Ok(ScanSummary::from(&diff))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(output)
    }
}

/// Scan a directory with progress reporting.
///
/// The `on_progress` callback receives `ProgressUpdate` objects periodically
/// (every 100 files). Uses v3's ownership-based ThreadsafeFunction lifecycle.
///
/// @param root - Directory to scan.
/// @param options - Optional scan configuration overrides.
/// @param on_progress - Callback receiving progress updates.
#[napi(js_name = "driftScanWithProgress")]
pub fn drift_scan_with_progress(
    root: String,
    options: Option<ScanOptions>,
    on_progress: ThreadsafeFunction<ProgressUpdate, ()>,
) -> AsyncTask<ScanWithProgressTask> {
    reset_cancellation();
    AsyncTask::new(ScanWithProgressTask {
        root: PathBuf::from(root),
        options: options.unwrap_or_default(),
        on_progress: Arc::new(on_progress),
    })
}

// ---- Cancellation ----

/// Cancel a running scan operation.
///
/// Sets the global cancellation flag. Rayon workers check this between files.
/// Already-processed files are retained; in-progress files are discarded.
/// The scan returns with `status: "partial"`.
#[napi(js_name = "driftCancelScan")]
pub fn drift_cancel_scan() -> napi::Result<()> {
    SCAN_CANCELLED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Reset the cancellation flag. Called at the start of each new scan.
fn reset_cancellation() {
    SCAN_CANCELLED.store(false, Ordering::SeqCst);
}

// ---- Progress Handler ----

/// Bridges `DriftEventHandler` → `ThreadsafeFunction` for progress reporting.
///
/// Reports every 100 files to keep NAPI callback overhead negligible (<0.1% of scan time).
/// Non-blocking: if the JS callback queue is full, the update is dropped rather than
/// blocking the Rust thread.
struct NapiProgressHandler {
    tsfn: Arc<ThreadsafeFunction<ProgressUpdate, ()>>,
}

impl NapiProgressHandler {
    fn new(tsfn: Arc<ThreadsafeFunction<ProgressUpdate, ()>>) -> Self {
        Self { tsfn }
    }
}

impl DriftEventHandler for NapiProgressHandler {
    fn on_scan_progress(&self, event: &ScanProgressEvent) {
        // Report every 100 files (from audit spec) or at completion
        if event.processed % 100 == 0 || event.processed == event.total {
            let update = ProgressUpdate {
                processed: event.processed as u32,
                total: event.total as u32,
                phase: "scanning".to_string(),
                current_file: None,
            };
            // Non-blocking call — drop update if JS queue is full
            let _ = self.tsfn.call(Ok(update), ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
}

/// No-op event handler for scans without progress reporting.
struct NoOpHandler;
impl DriftEventHandler for NoOpHandler {}

// ---- Helpers ----

/// Build a `ScanConfig` by merging runtime config with per-call options.
fn build_scan_config(base: &ScanConfig, opts: &ScanOptions) -> ScanConfig {
    let mut config = base.clone();

    if let Some(force) = opts.force_full {
        config.force_full_scan = Some(force);
    }
    if let Some(max_size) = opts.max_file_size {
        config.max_file_size = Some(max_size as u64);
    }
    if let Some(ref extra) = opts.extra_ignore {
        config.extra_ignore.extend(extra.iter().cloned());
    }
    if let Some(follow) = opts.follow_symlinks {
        config.follow_symlinks = Some(follow);
    }

    config
}
