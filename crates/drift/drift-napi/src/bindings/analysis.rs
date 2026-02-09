//! Phase 2 NAPI bindings — drift_analyze(), drift_call_graph(), drift_boundaries().

use napi_derive::napi;
use serde::{Deserialize, Serialize};

use crate::conversions::error_codes;
use crate::runtime;

/// Analysis result returned to TypeScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsAnalysisResult {
    pub file: String,
    pub language: String,
    pub matches: Vec<JsPatternMatch>,
    pub analysis_time_us: f64,
}

/// A pattern match returned to TypeScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsPatternMatch {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub pattern_id: String,
    pub confidence: f64,
    pub category: String,
    pub detection_method: String,
    pub matched_text: String,
    pub cwe_ids: Vec<u32>,
    pub owasp: Option<String>,
}

/// Call graph result returned to TypeScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsCallGraphResult {
    pub total_functions: u32,
    pub total_edges: u32,
    pub entry_points: u32,
    pub resolution_rate: f64,
    pub build_duration_ms: f64,
}

/// Boundary detection result returned to TypeScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsBoundaryResult {
    pub models: Vec<JsModelResult>,
    pub sensitive_fields: Vec<JsSensitiveField>,
    pub frameworks_detected: Vec<String>,
}

/// A model result returned to TypeScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsModelResult {
    pub name: String,
    pub table_name: Option<String>,
    pub file: String,
    pub framework: String,
    pub field_count: u32,
    pub confidence: f64,
}

/// A sensitive field result returned to TypeScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsSensitiveField {
    pub model_name: String,
    pub field_name: String,
    pub file: String,
    pub sensitivity: String,
    pub confidence: f64,
}

/// Run the full analysis pipeline on the project.
///
/// Returns analysis results for all files.
#[napi]
pub async fn drift_analyze() -> napi::Result<Vec<JsAnalysisResult>> {
    let _rt = runtime::get()?;

    // Phase 2 analysis pipeline would run here.
    // For now, return empty results — the pipeline integration
    // requires scan + parse results which come from the full orchestration.
    Ok(Vec::new())
}

/// Build or query the call graph.
#[napi]
pub async fn drift_call_graph() -> napi::Result<JsCallGraphResult> {
    let _rt = runtime::get()?;

    Ok(JsCallGraphResult {
        total_functions: 0,
        total_edges: 0,
        entry_points: 0,
        resolution_rate: 0.0,
        build_duration_ms: 0.0,
    })
}

/// Run boundary detection.
#[napi]
pub async fn drift_boundaries() -> napi::Result<JsBoundaryResult> {
    let _rt = runtime::get()?;

    Ok(JsBoundaryResult {
        models: Vec::new(),
        sensitive_fields: Vec::new(),
        frameworks_detected: Vec::new(),
    })
}
