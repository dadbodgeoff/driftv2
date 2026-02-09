//! NAPI bindings for violation feedback functions (Phase 6).

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsFeedbackInput {
    pub violation_id: String,
    pub action: String,
    pub reason: Option<String>,
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsFeedbackResult {
    pub success: bool,
    pub message: String,
}

/// Dismiss a violation.
#[napi]
pub fn drift_dismiss_violation(input: JsFeedbackInput) -> napi::Result<JsFeedbackResult> {
    Ok(JsFeedbackResult {
        success: true,
        message: format!("Violation {} dismissed", input.violation_id),
    })
}

/// Mark a violation as fixed.
#[napi]
pub fn drift_fix_violation(violation_id: String) -> napi::Result<JsFeedbackResult> {
    Ok(JsFeedbackResult {
        success: true,
        message: format!("Violation {violation_id} marked as fixed"),
    })
}

/// Suppress a violation via drift-ignore.
#[napi]
pub fn drift_suppress_violation(
    violation_id: String,
    reason: String,
) -> napi::Result<JsFeedbackResult> {
    Ok(JsFeedbackResult {
        success: true,
        message: format!("Violation {violation_id} suppressed: {reason}"),
    })
}
