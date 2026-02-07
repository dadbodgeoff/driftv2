//! Health bindings: getHealth, getMetrics, getDegradations.

use napi_derive::napi;
use serde_json::json;

use cortex_core::traits::IMemoryStorage;
use cortex_observability::HealthSnapshot;

use crate::conversions::{error_types, health_types};
use crate::runtime;

/// Get a comprehensive health report.
#[napi]
pub fn cortex_health_get_health() -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let mut obs = rt
        .observability
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Observability lock poisoned: {e}")))?;

    // Build a snapshot from current engine state.
    let type_counts = rt
        .storage
        .count_by_type()
        .map_err(error_types::to_napi_error)?;
    let total: usize = type_counts.iter().map(|(_, c)| c).sum();
    let avg_confidence = rt
        .storage
        .average_confidence()
        .map_err(error_types::to_napi_error)?;

    let snapshot = HealthSnapshot {
        total_memories: total,
        active_memories: total,
        archived_memories: 0,
        average_confidence: avg_confidence,
        db_size_bytes: 0,
        embedding_cache_hit_rate: 0.0,
        stale_count: 0,
        contradiction_count: 0,
        unresolved_contradictions: 0,
        consolidation_count: 0,
        memories_needing_validation: 0,
    };

    let report = obs
        .health_report(snapshot)
        .map_err(error_types::to_napi_error)?;
    health_types::health_report_to_json(&report)
}

/// Get system metrics as JSON.
#[napi]
pub fn cortex_health_get_metrics() -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let _obs = rt
        .observability
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Observability lock poisoned: {e}")))?;
    // MetricsCollector doesn't derive Serialize, so we build JSON manually.
    Ok(json!({
        "session_count": rt.session.session_count(),
        "causal_stats": {
            "node_count": rt.causal.stats().map(|(n, _)| n).unwrap_or(0),
            "edge_count": rt.causal.stats().map(|(_, e)| e).unwrap_or(0),
        },
    }))
}

/// Get degradation events and alerts.
#[napi]
pub fn cortex_health_get_degradations() -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let mut embeddings = rt
        .embeddings
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Embedding lock poisoned: {e}")))?;
    let events = embeddings.drain_degradation_events();
    health_types::degradation_events_to_json(&events)
}
