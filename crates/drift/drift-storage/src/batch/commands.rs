//! BatchCommand enum — all write operations that can be batched.

/// A command sent to the batch writer thread.
#[derive(Debug)]
pub enum BatchCommand {
    /// Insert or update file metadata rows.
    UpsertFileMetadata(Vec<FileMetadataRow>),
    /// Insert parse cache entries.
    InsertParseCache(Vec<ParseCacheRow>),
    /// Insert function rows.
    InsertFunctions(Vec<FunctionRow>),
    /// Delete file metadata for removed files.
    DeleteFileMetadata(Vec<String>),
    /// Flush any pending writes immediately.
    Flush,
    /// Shut down the writer thread.
    Shutdown,
    /// Insert call edge rows.
    InsertCallEdges(Vec<CallEdgeRow>),
    /// Insert detection rows.
    InsertDetections(Vec<DetectionRow>),
    /// Insert boundary rows.
    InsertBoundaries(Vec<BoundaryRow>),
    /// Insert pattern confidence rows.
    InsertPatternConfidence(Vec<PatternConfidenceRow>),
    /// Insert outlier rows.
    InsertOutliers(Vec<OutlierDetectionRow>),
    /// Insert convention rows.
    InsertConventions(Vec<ConventionInsertRow>),
}

/// A row for the file_metadata table.
#[derive(Debug, Clone)]
pub struct FileMetadataRow {
    pub path: String,
    pub language: Option<String>,
    pub file_size: i64,
    pub content_hash: Vec<u8>,
    pub mtime_secs: i64,
    pub mtime_nanos: i64,
    pub last_scanned_at: i64,
    pub scan_duration_us: Option<i64>,
}

/// A row for the parse_cache table.
#[derive(Debug, Clone)]
pub struct ParseCacheRow {
    pub content_hash: Vec<u8>,
    pub language: String,
    pub parse_result_json: String,
    pub created_at: i64,
}

/// A row for the functions table.
#[derive(Debug, Clone)]
pub struct FunctionRow {
    pub file: String,
    pub name: String,
    pub qualified_name: Option<String>,
    pub language: String,
    pub line: i64,
    pub end_line: i64,
    pub parameter_count: i64,
    pub return_type: Option<String>,
    pub is_exported: bool,
    pub is_async: bool,
    pub body_hash: Vec<u8>,
    pub signature_hash: Vec<u8>,
}

/// A row for the call_edges table.
#[derive(Debug, Clone)]
pub struct CallEdgeRow {
    pub caller_id: i64,
    pub callee_id: i64,
    pub resolution: String,
    pub confidence: f64,
    pub call_site_line: i64,
}

/// A row for the detections table.
#[derive(Debug, Clone)]
pub struct DetectionRow {
    pub file: String,
    pub line: i64,
    pub column_num: i64,
    pub pattern_id: String,
    pub category: String,
    pub confidence: f64,
    pub detection_method: String,
    pub cwe_ids: Option<String>,
    pub owasp: Option<String>,
    pub matched_text: Option<String>,
}

/// A row for the boundaries table.
#[derive(Debug, Clone)]
pub struct BoundaryRow {
    pub file: String,
    pub framework: String,
    pub model_name: String,
    pub table_name: Option<String>,
    pub field_name: Option<String>,
    pub sensitivity: Option<String>,
    pub confidence: f64,
}

/// A row for the pattern_confidence table.
#[derive(Debug, Clone)]
pub struct PatternConfidenceRow {
    pub pattern_id: String,
    pub alpha: f64,
    pub beta: f64,
    pub posterior_mean: f64,
    pub credible_interval_low: f64,
    pub credible_interval_high: f64,
    pub tier: String,
    pub momentum: String,
}

/// A row for the outliers table.
#[derive(Debug, Clone)]
pub struct OutlierDetectionRow {
    pub pattern_id: String,
    pub file: String,
    pub line: i64,
    pub deviation_score: f64,
    pub significance: String,
    pub method: String,
}

/// A row for the conventions table.
#[derive(Debug, Clone)]
pub struct ConventionInsertRow {
    pub pattern_id: String,
    pub category: String,
    pub scope: String,
    pub dominance_ratio: f64,
    pub promotion_status: String,
    pub discovered_at: i64,
    pub last_seen: i64,
    pub expires_at: Option<i64>,
}
