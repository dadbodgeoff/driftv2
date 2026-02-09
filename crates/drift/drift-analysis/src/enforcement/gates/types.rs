//! Core types for quality gates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// The 6 quality gate identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateId {
    PatternCompliance,
    ConstraintVerification,
    SecurityBoundaries,
    TestCoverage,
    ErrorHandling,
    Regression,
}

impl GateId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PatternCompliance => "pattern-compliance",
            Self::ConstraintVerification => "constraint-verification",
            Self::SecurityBoundaries => "security-boundaries",
            Self::TestCoverage => "test-coverage",
            Self::ErrorHandling => "error-handling",
            Self::Regression => "regression",
        }
    }

    pub fn all() -> &'static [GateId] {
        &[
            Self::PatternCompliance,
            Self::ConstraintVerification,
            Self::SecurityBoundaries,
            Self::TestCoverage,
            Self::ErrorHandling,
            Self::Regression,
        ]
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Gate execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateStatus {
    Passed,
    Failed,
    Warned,
    Skipped,
    Errored,
}

/// Result produced by each gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_id: GateId,
    pub status: GateStatus,
    pub passed: bool,
    pub score: f64,
    pub summary: String,
    pub violations: Vec<super::super::rules::Violation>,
    pub warnings: Vec<String>,
    pub execution_time_ms: u64,
    pub details: serde_json::Value,
    pub error: Option<String>,
}

impl GateResult {
    /// Create a passing gate result.
    pub fn pass(gate_id: GateId, score: f64, summary: String) -> Self {
        Self {
            gate_id,
            status: GateStatus::Passed,
            passed: true,
            score,
            summary,
            violations: Vec::new(),
            warnings: Vec::new(),
            execution_time_ms: 0,
            details: serde_json::Value::Null,
            error: None,
        }
    }

    /// Create a failing gate result.
    pub fn fail(
        gate_id: GateId,
        score: f64,
        summary: String,
        violations: Vec<super::super::rules::Violation>,
    ) -> Self {
        Self {
            gate_id,
            status: GateStatus::Failed,
            passed: false,
            score,
            summary,
            violations,
            warnings: Vec::new(),
            execution_time_ms: 0,
            details: serde_json::Value::Null,
            error: None,
        }
    }

    /// Create a warned gate result.
    pub fn warn(gate_id: GateId, score: f64, summary: String, warnings: Vec<String>) -> Self {
        Self {
            gate_id,
            status: GateStatus::Warned,
            passed: true,
            score,
            summary,
            violations: Vec::new(),
            warnings,
            execution_time_ms: 0,
            details: serde_json::Value::Null,
            error: None,
        }
    }

    /// Create a skipped gate result.
    pub fn skipped(gate_id: GateId, reason: String) -> Self {
        Self {
            gate_id,
            status: GateStatus::Skipped,
            passed: true,
            score: 0.0,
            summary: reason,
            violations: Vec::new(),
            warnings: Vec::new(),
            execution_time_ms: 0,
            details: serde_json::Value::Null,
            error: None,
        }
    }

    /// Create an errored gate result.
    pub fn errored(gate_id: GateId, error: String) -> Self {
        Self {
            gate_id,
            status: GateStatus::Errored,
            passed: false,
            score: 0.0,
            summary: format!("Gate errored: {error}"),
            violations: Vec::new(),
            warnings: Vec::new(),
            execution_time_ms: 0,
            details: serde_json::Value::Null,
            error: Some(error),
        }
    }
}

/// Input provided to each gate by the orchestrator.
#[derive(Debug, Clone, Default)]
pub struct GateInput {
    pub files: Vec<String>,
    pub all_files: Vec<String>,
    pub patterns: Vec<super::super::rules::PatternInfo>,
    pub constraints: Vec<ConstraintInput>,
    pub security_findings: Vec<SecurityFindingInput>,
    pub test_coverage: Option<TestCoverageInput>,
    pub error_gaps: Vec<ErrorGapInput>,
    pub previous_health_score: Option<f64>,
    pub current_health_score: Option<f64>,
    pub predecessor_results: HashMap<GateId, GateResult>,
}

/// Constraint data for the constraint verification gate.
#[derive(Debug, Clone)]
pub struct ConstraintInput {
    pub id: String,
    pub description: String,
    pub passed: bool,
    pub violations: Vec<ConstraintViolationInput>,
}

#[derive(Debug, Clone)]
pub struct ConstraintViolationInput {
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
}

/// Security finding data for the security boundaries gate.
#[derive(Debug, Clone)]
pub struct SecurityFindingInput {
    pub file: String,
    pub line: u32,
    pub description: String,
    pub severity: String,
    pub cwe_ids: Vec<u32>,
    pub owasp_categories: Vec<String>,
}

/// Test coverage data for the test coverage gate.
#[derive(Debug, Clone)]
pub struct TestCoverageInput {
    pub overall_coverage: f64,
    pub threshold: f64,
    pub uncovered_files: Vec<String>,
}

/// Error handling gap data for the error handling gate.
#[derive(Debug, Clone)]
pub struct ErrorGapInput {
    pub file: String,
    pub line: u32,
    pub gap_type: String,
    pub message: String,
}

/// Gate dependency specification.
#[derive(Debug, Clone)]
pub struct GateDependency {
    pub gate_id: GateId,
    pub depends_on: Vec<GateId>,
}

/// Trait for quality gate implementations.
pub trait QualityGate: Send + Sync {
    fn id(&self) -> GateId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn evaluate(&self, input: &GateInput) -> GateResult;
    fn dependencies(&self) -> Vec<GateId> {
        Vec::new()
    }
}
