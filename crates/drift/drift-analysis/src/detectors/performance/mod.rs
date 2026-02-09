//! Performance detector — N+1 query patterns, unnecessary allocations, hot paths.

use smallvec::SmallVec;

use crate::detectors::traits::{Detector, DetectorCategory, DetectorVariant};
use crate::engine::types::{DetectionMethod, PatternCategory, PatternMatch};
use crate::engine::visitor::DetectionContext;

pub struct PerformanceDetector;

impl Detector for PerformanceDetector {
    fn id(&self) -> &str { "performance-base" }
    fn category(&self) -> DetectorCategory { DetectorCategory::Performance }
    fn variant(&self) -> DetectorVariant { DetectorVariant::Base }

    fn detect(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        // Detect potential N+1 patterns: database/fetch calls inside functions with loops
        let db_callees = ["query", "find", "findone", "findall", "select", "fetch",
                          "execute", "get", "load", "read"];
        for func in ctx.functions {
            let calls_in_func: Vec<_> = ctx.call_sites.iter()
                .filter(|c| c.line >= func.line && c.line <= func.end_line)
                .collect();
            let db_calls: Vec<_> = calls_in_func.iter()
                .filter(|c| db_callees.contains(&c.callee_name.to_lowercase().as_str()))
                .collect();
            if db_calls.len() > 1 {
                matches.push(PatternMatch {
                    file: ctx.file.to_string(),
                    line: func.line,
                    column: func.column,
                    pattern_id: "PERF-N1-001".to_string(),
                    confidence: 0.60,
                    cwe_ids: SmallVec::new(),
                    owasp: None,
                    detection_method: DetectionMethod::AstVisitor,
                    category: PatternCategory::Performance,
                    matched_text: format!(
                        "Potential N+1 in {}: {} DB-like calls",
                        func.name, db_calls.len()
                    ),
                });
            }
        }

        // Detect unnecessary allocation patterns (clone, collect, to_vec in hot paths)
        let alloc_callees = ["clone", "to_vec", "to_string", "to_owned", "collect"];
        for call in ctx.call_sites {
            let callee_lower = call.callee_name.to_lowercase();
            if alloc_callees.contains(&callee_lower.as_str()) {
                matches.push(PatternMatch {
                    file: ctx.file.to_string(),
                    line: call.line,
                    column: call.column,
                    pattern_id: "PERF-ALLOC-002".to_string(),
                    confidence: 0.50,
                    cwe_ids: SmallVec::new(),
                    owasp: None,
                    detection_method: DetectionMethod::AstVisitor,
                    category: PatternCategory::Performance,
                    matched_text: format!("Allocation: {}", call.callee_name),
                });
            }
        }

        // Detect async functions without await (potential missing concurrency)
        for func in ctx.functions {
            if func.is_async {
                let has_await = ctx.call_sites.iter().any(|c| {
                    c.is_await && c.line >= func.line && c.line <= func.end_line
                });
                if !has_await {
                    matches.push(PatternMatch {
                        file: ctx.file.to_string(),
                        line: func.line,
                        column: func.column,
                        pattern_id: "PERF-ASYNC-003".to_string(),
                        confidence: 0.65,
                        cwe_ids: SmallVec::new(),
                        owasp: None,
                        detection_method: DetectionMethod::AstVisitor,
                        category: PatternCategory::Performance,
                        matched_text: format!("Async function without await: {}", func.name),
                    });
                }
            }
        }

        matches
    }
}
