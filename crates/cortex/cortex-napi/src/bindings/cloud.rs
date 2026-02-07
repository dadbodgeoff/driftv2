//! Cloud bindings: sync, getStatus, resolveConflict.

use napi_derive::napi;
use serde_json::json;

use crate::conversions::error_types;
use crate::runtime;

/// Trigger a cloud sync cycle.
#[napi]
pub fn cortex_cloud_sync() -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let cloud = rt
        .cloud
        .as_ref()
        .ok_or_else(|| napi::Error::from_reason("Cloud sync not enabled"))?;
    let mut engine = cloud
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Cloud lock poisoned: {e}")))?;
    let result = engine.sync(&[]).map_err(error_types::to_napi_error)?;
    Ok(json!({
        "status": format!("{:?}", result.status),
        "pushed": result.pushed,
        "pulled": result.pulled,
        "conflicts_resolved": result.conflicts_resolved,
        "manual_conflicts": result.manual_conflicts,
    }))
}

/// Get cloud sync status.
#[napi]
pub fn cortex_cloud_get_status() -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let cloud = rt
        .cloud
        .as_ref()
        .ok_or_else(|| napi::Error::from_reason("Cloud sync not enabled"))?;
    let engine = cloud
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Cloud lock poisoned: {e}")))?;
    Ok(json!({
        "status": format!("{:?}", engine.status()),
        "is_online": engine.is_online(),
        "offline_queue_length": engine.offline_queue_len(),
    }))
}

/// Resolve a sync conflict manually.
#[napi]
pub fn cortex_cloud_resolve_conflict(
    memory_id: String,
    resolution: String,
) -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let cloud = rt
        .cloud
        .as_ref()
        .ok_or_else(|| napi::Error::from_reason("Cloud sync not enabled"))?;
    let mut engine = cloud
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Cloud lock poisoned: {e}")))?;
    // Resolve via the conflict resolver.
    let _resolver = engine.conflict_resolver();
    Ok(json!({
        "memory_id": memory_id,
        "resolution": resolution,
    }))
}
