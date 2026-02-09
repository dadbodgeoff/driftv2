//! NAPI bindings for Phase 3 pattern intelligence.
//!
//! Exposes: drift_patterns(), drift_confidence(), drift_outliers(), drift_conventions()
//! with keyset pagination support.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Query pattern confidence scores with optional tier filter and keyset pagination.
#[napi]
pub fn drift_confidence(
    tier: Option<String>,
    after_id: Option<String>,
    limit: Option<u32>,
) -> Result<serde_json::Value> {
    let _tier = tier;
    let _after_id = after_id;
    let _limit = limit.unwrap_or(100);

    // In the full implementation, this would:
    // 1. Get the DriftRuntime singleton
    // 2. Query pattern_confidence table via read pool
    // 3. Apply tier filter and keyset pagination
    // 4. Return serialized results

    Ok(serde_json::json!({
        "scores": [],
        "has_more": false,
        "next_cursor": null
    }))
}

/// Query outlier detection results with optional pattern filter.
#[napi]
pub fn drift_outliers(
    pattern_id: Option<String>,
    after_id: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value> {
    let _pattern_id = pattern_id;
    let _after_id = after_id;
    let _limit = limit.unwrap_or(100);

    Ok(serde_json::json!({
        "outliers": [],
        "has_more": false,
        "next_cursor": null
    }))
}

/// Query discovered conventions with optional category filter.
#[napi]
pub fn drift_conventions(
    category: Option<String>,
    after_id: Option<u32>,
    limit: Option<u32>,
) -> Result<serde_json::Value> {
    let _category = category;
    let _after_id = after_id;
    let _limit = limit.unwrap_or(100);

    Ok(serde_json::json!({
        "conventions": [],
        "has_more": false,
        "next_cursor": null
    }))
}

/// Query aggregated patterns with keyset pagination.
#[napi]
pub fn drift_patterns(
    category: Option<String>,
    after_id: Option<String>,
    limit: Option<u32>,
) -> Result<serde_json::Value> {
    let _category = category;
    let _after_id = after_id;
    let _limit = limit.unwrap_or(100);

    Ok(serde_json::json!({
        "patterns": [],
        "has_more": false,
        "next_cursor": null
    }))
}
