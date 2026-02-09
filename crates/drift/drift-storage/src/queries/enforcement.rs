//! Queries for enforcement tables: violations, gate_results, audit_snapshots,
//! health_trends, feedback.

use rusqlite::{params, Connection};

use drift_core::errors::StorageError;

// ─── Row Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ViolationRow {
    pub id: String,
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
    pub severity: String,
    pub pattern_id: String,
    pub rule_id: String,
    pub message: String,
    pub cwe_id: Option<u32>,
    pub owasp_category: Option<String>,
    pub suppressed: bool,
}

#[derive(Debug, Clone)]
pub struct GateResultRow {
    pub gate_id: String,
    pub status: String,
    pub passed: bool,
    pub score: f64,
    pub summary: String,
    pub violation_count: u32,
    pub execution_time_ms: u64,
    pub details: Option<String>,
    pub run_at: u64,
}

#[derive(Debug, Clone)]
pub struct AuditSnapshotRow {
    pub health_score: f64,
    pub avg_confidence: f64,
    pub approval_ratio: f64,
    pub compliance_rate: f64,
    pub cross_validation_rate: f64,
    pub duplicate_free_rate: f64,
    pub pattern_count: u32,
    pub category_scores: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct HealthTrendRow {
    pub metric_name: String,
    pub metric_value: f64,
    pub recorded_at: u64,
}

#[derive(Debug, Clone)]
pub struct FeedbackRow {
    pub violation_id: String,
    pub pattern_id: String,
    pub detector_id: String,
    pub action: String,
    pub dismissal_reason: Option<String>,
    pub reason: Option<String>,
    pub author: Option<String>,
    pub created_at: u64,
}

// ─── Violations ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn insert_violation(
    conn: &Connection,
    v: &ViolationRow,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT OR REPLACE INTO violations (id, file, line, column_num, severity, pattern_id, rule_id, message, cwe_id, owasp_category, suppressed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![v.id, v.file, v.line, v.column, v.severity, v.pattern_id, v.rule_id, v.message, v.cwe_id, v.owasp_category, v.suppressed as i32],
    ).map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    Ok(())
}

pub fn query_violations_by_file(
    conn: &Connection,
    file: &str,
) -> Result<Vec<ViolationRow>, StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, file, line, column_num, severity, pattern_id, rule_id, message, cwe_id, owasp_category, suppressed
             FROM violations WHERE file = ?1 ORDER BY line",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    let rows = stmt
        .query_map(params![file], |row| {
            Ok(ViolationRow {
                id: row.get(0)?,
                file: row.get(1)?,
                line: row.get(2)?,
                column: row.get(3)?,
                severity: row.get(4)?,
                pattern_id: row.get(5)?,
                rule_id: row.get(6)?,
                message: row.get(7)?,
                cwe_id: row.get(8)?,
                owasp_category: row.get(9)?,
                suppressed: row.get::<_, i32>(10)? != 0,
            })
        })
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })
}

pub fn query_all_violations(conn: &Connection) -> Result<Vec<ViolationRow>, StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, file, line, column_num, severity, pattern_id, rule_id, message, cwe_id, owasp_category, suppressed
             FROM violations ORDER BY file, line",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ViolationRow {
                id: row.get(0)?,
                file: row.get(1)?,
                line: row.get(2)?,
                column: row.get(3)?,
                severity: row.get(4)?,
                pattern_id: row.get(5)?,
                rule_id: row.get(6)?,
                message: row.get(7)?,
                cwe_id: row.get(8)?,
                owasp_category: row.get(9)?,
                suppressed: row.get::<_, i32>(10)? != 0,
            })
        })
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })
}

// ─── Gate Results ────────────────────────────────────────────────────

pub fn insert_gate_result(
    conn: &Connection,
    g: &GateResultRow,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO gate_results (gate_id, status, passed, score, summary, violation_count, execution_time_ms, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![g.gate_id, g.status, g.passed as i32, g.score, g.summary, g.violation_count, g.execution_time_ms, g.details],
    ).map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    Ok(())
}

pub fn query_gate_results(conn: &Connection) -> Result<Vec<GateResultRow>, StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT gate_id, status, passed, score, summary, violation_count, execution_time_ms, details, run_at
             FROM gate_results ORDER BY run_at DESC",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    let rows = stmt
        .query_map([], |row| {
            Ok(GateResultRow {
                gate_id: row.get(0)?,
                status: row.get(1)?,
                passed: row.get::<_, i32>(2)? != 0,
                score: row.get(3)?,
                summary: row.get(4)?,
                violation_count: row.get(5)?,
                execution_time_ms: row.get(6)?,
                details: row.get(7)?,
                run_at: row.get(8)?,
            })
        })
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })
}

// ─── Audit Snapshots ─────────────────────────────────────────────────

pub fn insert_audit_snapshot(
    conn: &Connection,
    s: &AuditSnapshotRow,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO audit_snapshots (health_score, avg_confidence, approval_ratio, compliance_rate, cross_validation_rate, duplicate_free_rate, pattern_count, category_scores)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![s.health_score, s.avg_confidence, s.approval_ratio, s.compliance_rate, s.cross_validation_rate, s.duplicate_free_rate, s.pattern_count, s.category_scores],
    ).map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    Ok(())
}

pub fn query_audit_snapshots(
    conn: &Connection,
    limit: u32,
) -> Result<Vec<AuditSnapshotRow>, StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT health_score, avg_confidence, approval_ratio, compliance_rate, cross_validation_rate, duplicate_free_rate, pattern_count, category_scores, created_at
             FROM audit_snapshots ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(AuditSnapshotRow {
                health_score: row.get(0)?,
                avg_confidence: row.get(1)?,
                approval_ratio: row.get(2)?,
                compliance_rate: row.get(3)?,
                cross_validation_rate: row.get(4)?,
                duplicate_free_rate: row.get(5)?,
                pattern_count: row.get(6)?,
                category_scores: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })
}

// ─── Health Trends ───────────────────────────────────────────────────

pub fn insert_health_trend(
    conn: &Connection,
    metric_name: &str,
    metric_value: f64,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO health_trends (metric_name, metric_value) VALUES (?1, ?2)",
        params![metric_name, metric_value],
    ).map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    Ok(())
}

pub fn query_health_trends(
    conn: &Connection,
    metric_name: &str,
    limit: u32,
) -> Result<Vec<HealthTrendRow>, StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT metric_name, metric_value, recorded_at
             FROM health_trends WHERE metric_name = ?1 ORDER BY recorded_at DESC LIMIT ?2",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    let rows = stmt
        .query_map(params![metric_name, limit], |row| {
            Ok(HealthTrendRow {
                metric_name: row.get(0)?,
                metric_value: row.get(1)?,
                recorded_at: row.get(2)?,
            })
        })
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })
}

// ─── Feedback ────────────────────────────────────────────────────────

pub fn insert_feedback(
    conn: &Connection,
    f: &FeedbackRow,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO feedback (violation_id, pattern_id, detector_id, action, dismissal_reason, reason, author)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![f.violation_id, f.pattern_id, f.detector_id, f.action, f.dismissal_reason, f.reason, f.author],
    ).map_err(|e| StorageError::SqliteError { message: e.to_string() })?;
    Ok(())
}

pub fn query_feedback_by_detector(
    conn: &Connection,
    detector_id: &str,
) -> Result<Vec<FeedbackRow>, StorageError> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT violation_id, pattern_id, detector_id, action, dismissal_reason, reason, author, created_at
             FROM feedback WHERE detector_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    let rows = stmt
        .query_map(params![detector_id], |row| {
            Ok(FeedbackRow {
                violation_id: row.get(0)?,
                pattern_id: row.get(1)?,
                detector_id: row.get(2)?,
                action: row.get(3)?,
                dismissal_reason: row.get(4)?,
                reason: row.get(5)?,
                author: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::SqliteError { message: e.to_string() })
}
