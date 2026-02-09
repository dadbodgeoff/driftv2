//! Rules evaluator — maps detected patterns + outliers to actionable violations.

use super::quick_fixes::QuickFixGenerator;
use super::suppression::SuppressionChecker;
use super::types::*;

/// The rules evaluator maps patterns and outliers to violations with severity and quick fixes.
pub struct RulesEvaluator {
    fix_generator: QuickFixGenerator,
    suppression_checker: SuppressionChecker,
}

impl RulesEvaluator {
    pub fn new() -> Self {
        Self {
            fix_generator: QuickFixGenerator::new(),
            suppression_checker: SuppressionChecker::new(),
        }
    }

    /// Evaluate all patterns and produce violations.
    pub fn evaluate(&self, input: &RulesInput) -> Vec<Violation> {
        let mut violations = Vec::new();

        for pattern in &input.patterns {
            // Map outliers to violations (deviations from the pattern)
            for outlier in &pattern.outliers {
                let severity = self.assign_severity(pattern, outlier);
                let rule_id = format!("{}/{}", pattern.category, pattern.pattern_id);
                let id = format!("{}-{}-{}", rule_id, outlier.file, outlier.line);

                let quick_fix = self.fix_generator.suggest(pattern, outlier);

                let suppressed = self.suppression_checker.is_suppressed(
                    &outlier.file,
                    outlier.line,
                    Some(&rule_id),
                    &input.source_lines,
                );

                violations.push(Violation {
                    id,
                    file: outlier.file.clone(),
                    line: outlier.line,
                    column: outlier.column,
                    end_line: None,
                    end_column: None,
                    severity,
                    pattern_id: pattern.pattern_id.clone(),
                    rule_id,
                    message: outlier.message.clone(),
                    quick_fix,
                    cwe_id: pattern.cwe_ids.first().copied(),
                    owasp_category: pattern.owasp_categories.first().cloned(),
                    suppressed,
                    is_new: false,
                });
            }
        }

        // Deduplicate: same file+line+rule_id → keep highest severity
        self.deduplicate(&mut violations);
        violations
    }

    /// Assign severity based on pattern category and CWE mapping.
    fn assign_severity(&self, pattern: &PatternInfo, outlier: &OutlierLocation) -> Severity {
        // Security-related patterns with CWE IDs → Error
        if !pattern.cwe_ids.is_empty() {
            return match pattern.cwe_ids[0] {
                // CWE-89 SQL injection, CWE-79 XSS, CWE-78 OS command injection
                89 | 79 | 78 | 22 | 94 | 502 | 611 | 918 | 327 | 798 => Severity::Error,
                _ => Severity::Warning,
            };
        }

        // Category-based severity
        match pattern.category.as_str() {
            "security" | "taint" | "crypto" => Severity::Error,
            "error_handling" | "constraint" | "boundary" => Severity::Warning,
            "naming" | "convention" | "style" => {
                if outlier.deviation_score > 3.0 {
                    Severity::Warning
                } else {
                    Severity::Info
                }
            }
            "documentation" => Severity::Info,
            _ => {
                if outlier.deviation_score > 3.0 {
                    Severity::Warning
                } else {
                    Severity::Info
                }
            }
        }
    }

    /// Deduplicate violations: same file+line from multiple detectors → keep highest severity.
    fn deduplicate(&self, violations: &mut Vec<Violation>) {
        violations.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.line.cmp(&b.line))
                .then(a.severity.cmp(&b.severity))
        });

        let mut seen = std::collections::HashSet::new();
        violations.retain(|v| {
            let key = format!("{}:{}:{}", v.file, v.line, v.rule_id);
            seen.insert(key)
        });
    }
}

impl Default for RulesEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
