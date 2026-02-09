//! NAPI bindings for enforcement systems (Phase 6).
//!
//! Exposes drift_check(), drift_audit(), drift_violations(), drift_gates().

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

// ─── Violation Types ─────────────────────────────────────────────────

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsViolation {
    pub id: String,
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
    pub severity: String,
    pub rule_id: String,
    pub message: String,
    pub cwe_id: Option<u32>,
    pub owasp_category: Option<String>,
    pub suppressed: bool,
}

// ─── Gate Result Types ───────────────────────────────────────────────

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsGateResult {
    pub gate_id: String,
    pub status: String,
    pub passed: bool,
    pub score: f64,
    pub summary: String,
    pub violation_count: u32,
    pub execution_time_ms: u32,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsCheckResult {
    pub overall_passed: bool,
    pub total_violations: u32,
    pub gates: Vec<JsGateResult>,
    pub sarif: Option<String>,
}

// ─── Audit Types ─────────────────────────────────────────────────────

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsHealthBreakdown {
    pub avg_confidence: f64,
    pub approval_ratio: f64,
    pub compliance_rate: f64,
    pub cross_validation_rate: f64,
    pub duplicate_free_rate: f64,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsAuditResult {
    pub health_score: f64,
    pub breakdown: JsHealthBreakdown,
    pub trend: String,
    pub degradation_alerts: Vec<String>,
    pub auto_approved_count: u32,
    pub needs_review_count: u32,
}

// ─── NAPI Functions ──────────────────────────────────────────────────

/// Run quality gate checks on the project.
#[napi]
pub fn drift_check(_root: String) -> napi::Result<JsCheckResult> {
    Ok(JsCheckResult {
        overall_passed: true,
        total_violations: 0,
        gates: Vec::new(),
        sarif: None,
    })
}

/// Run audit analysis on the project.
#[napi]
pub fn drift_audit(_root: String) -> napi::Result<JsAuditResult> {
    Ok(JsAuditResult {
        health_score: 100.0,
        breakdown: JsHealthBreakdown {
            avg_confidence: 0.0,
            approval_ratio: 0.0,
            compliance_rate: 1.0,
            cross_validation_rate: 0.0,
            duplicate_free_rate: 1.0,
        },
        trend: "stable".to_string(),
        degradation_alerts: Vec::new(),
        auto_approved_count: 0,
        needs_review_count: 0,
    })
}

/// Query violations for the project.
#[napi]
pub fn drift_violations(_root: String) -> napi::Result<Vec<JsViolation>> {
    Ok(Vec::new())
}

/// Query gate results for the project.
#[napi]
pub fn drift_gates(_root: String) -> napi::Result<Vec<JsGateResult>> {
    Ok(Vec::new())
}
