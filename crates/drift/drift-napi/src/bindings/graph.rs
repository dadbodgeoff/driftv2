//! NAPI bindings for all 5 graph intelligence systems.
//!
//! Exposes reachability, taint, error handling, impact, and test topology
//! analysis functions to TypeScript/JavaScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

// --- Reachability ---

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsReachabilityResult {
    pub source: String,
    pub reachable_count: u32,
    pub sensitivity: String,
    pub max_depth: u32,
    pub engine: String,
}

#[napi]
pub fn drift_reachability(
    function_key: String,
    direction: String,
) -> napi::Result<JsReachabilityResult> {
    // Stub: would use DriftRuntime to access call graph
    Ok(JsReachabilityResult {
        source: function_key,
        reachable_count: 0,
        sensitivity: "low".to_string(),
        max_depth: 0,
        engine: "petgraph".to_string(),
    })
}

// --- Taint Analysis ---

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsTaintFlow {
    pub source_file: String,
    pub source_line: u32,
    pub source_type: String,
    pub sink_file: String,
    pub sink_line: u32,
    pub sink_type: String,
    pub cwe_id: Option<u32>,
    pub is_sanitized: bool,
    pub confidence: f64,
    pub path_length: u32,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsTaintResult {
    pub flows: Vec<JsTaintFlow>,
    pub vulnerability_count: u32,
    pub source_count: u32,
    pub sink_count: u32,
}

#[napi]
pub fn drift_taint_analysis(root: String) -> napi::Result<JsTaintResult> {
    // Stub: would run full taint analysis pipeline
    Ok(JsTaintResult {
        flows: Vec::new(),
        vulnerability_count: 0,
        source_count: 0,
        sink_count: 0,
    })
}

// --- Error Handling ---

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsErrorGap {
    pub file: String,
    pub function_name: String,
    pub line: u32,
    pub gap_type: String,
    pub severity: String,
    pub cwe_id: Option<u32>,
    pub remediation: Option<String>,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsErrorHandlingResult {
    pub gaps: Vec<JsErrorGap>,
    pub handler_count: u32,
    pub unhandled_count: u32,
}

#[napi]
pub fn drift_error_handling(root: String) -> napi::Result<JsErrorHandlingResult> {
    Ok(JsErrorHandlingResult {
        gaps: Vec::new(),
        handler_count: 0,
        unhandled_count: 0,
    })
}

// --- Impact Analysis ---

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsBlastRadius {
    pub function_id: String,
    pub caller_count: u32,
    pub risk_score: f64,
    pub max_depth: u32,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsDeadCode {
    pub function_id: String,
    pub reason: String,
    pub exclusion: Option<String>,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsImpactResult {
    pub blast_radii: Vec<JsBlastRadius>,
    pub dead_code: Vec<JsDeadCode>,
}

#[napi]
pub fn drift_impact_analysis(root: String) -> napi::Result<JsImpactResult> {
    Ok(JsImpactResult {
        blast_radii: Vec::new(),
        dead_code: Vec::new(),
    })
}

// --- Test Topology ---

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsTestQuality {
    pub coverage_breadth: f64,
    pub coverage_depth: f64,
    pub assertion_density: f64,
    pub mock_ratio: f64,
    pub isolation: f64,
    pub freshness: f64,
    pub stability: f64,
    pub overall: f64,
    pub smell_count: u32,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsTestTopologyResult {
    pub quality: JsTestQuality,
    pub test_count: u32,
    pub source_count: u32,
    pub coverage_percent: f64,
    pub minimum_test_set_size: u32,
}

#[napi]
pub fn drift_test_topology(root: String) -> napi::Result<JsTestTopologyResult> {
    Ok(JsTestTopologyResult {
        quality: JsTestQuality {
            coverage_breadth: 0.0,
            coverage_depth: 0.0,
            assertion_density: 0.0,
            mock_ratio: 0.0,
            isolation: 1.0,
            freshness: 1.0,
            stability: 1.0,
            overall: 0.0,
            smell_count: 0,
        },
        test_count: 0,
        source_count: 0,
        coverage_percent: 0.0,
        minimum_test_set_size: 0,
    })
}
