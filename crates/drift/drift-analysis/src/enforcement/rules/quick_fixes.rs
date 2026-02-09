//! Quick-fix generator — 7 fix strategies for violations.

use super::types::*;

/// Generates quick-fix suggestions for violations.
pub struct QuickFixGenerator;

impl QuickFixGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Suggest a quick fix for an outlier based on its pattern category.
    pub fn suggest(&self, pattern: &PatternInfo, outlier: &OutlierLocation) -> Option<QuickFix> {
        let strategy = self.select_strategy(pattern, outlier)?;
        let description = self.describe_fix(&strategy, pattern, outlier);
        let replacement = self.generate_replacement(&strategy, pattern, outlier);

        Some(QuickFix {
            strategy,
            description,
            replacement,
        })
    }

    /// Select the appropriate fix strategy based on pattern category.
    fn select_strategy(
        &self,
        pattern: &PatternInfo,
        _outlier: &OutlierLocation,
    ) -> Option<QuickFixStrategy> {
        match pattern.category.as_str() {
            "naming" | "convention" => Some(QuickFixStrategy::Rename),
            "error_handling" => Some(QuickFixStrategy::WrapInTryCatch),
            "import" | "dependency" => Some(QuickFixStrategy::AddImport),
            "type_safety" => Some(QuickFixStrategy::AddTypeAnnotation),
            "documentation" => Some(QuickFixStrategy::AddDocumentation),
            "test_coverage" => Some(QuickFixStrategy::AddTest),
            "complexity" | "decomposition" => Some(QuickFixStrategy::ExtractFunction),
            "security" | "taint" | "crypto" => Some(QuickFixStrategy::WrapInTryCatch),
            _ => None,
        }
    }

    /// Generate a human-readable description of the fix.
    fn describe_fix(
        &self,
        strategy: &QuickFixStrategy,
        pattern: &PatternInfo,
        _outlier: &OutlierLocation,
    ) -> String {
        match strategy {
            QuickFixStrategy::AddImport => {
                format!("Add missing import for pattern '{}'", pattern.pattern_id)
            }
            QuickFixStrategy::Rename => {
                format!(
                    "Rename to match '{}' convention pattern",
                    pattern.pattern_id
                )
            }
            QuickFixStrategy::ExtractFunction => {
                "Extract complex logic into a separate function".to_string()
            }
            QuickFixStrategy::WrapInTryCatch => {
                "Wrap in try/catch block for proper error handling".to_string()
            }
            QuickFixStrategy::AddTypeAnnotation => {
                "Add type annotation for type safety".to_string()
            }
            QuickFixStrategy::AddTest => {
                format!("Add test coverage for pattern '{}'", pattern.pattern_id)
            }
            QuickFixStrategy::AddDocumentation => {
                "Add documentation comment".to_string()
            }
        }
    }

    /// Generate replacement text for the fix (if applicable).
    fn generate_replacement(
        &self,
        strategy: &QuickFixStrategy,
        _pattern: &PatternInfo,
        _outlier: &OutlierLocation,
    ) -> Option<String> {
        match strategy {
            QuickFixStrategy::WrapInTryCatch => {
                Some("try {\n  // existing code\n} catch (error) {\n  // handle error\n}".to_string())
            }
            QuickFixStrategy::AddDocumentation => {
                Some("/** TODO: Add documentation */".to_string())
            }
            QuickFixStrategy::AddTypeAnnotation => {
                Some(": unknown".to_string())
            }
            _ => None,
        }
    }
}

impl Default for QuickFixGenerator {
    fn default() -> Self {
        Self::new()
    }
}
