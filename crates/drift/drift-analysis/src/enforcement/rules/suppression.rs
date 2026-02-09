//! Inline suppression system — `drift-ignore` comments.

use std::collections::HashMap;

/// Checks whether violations are suppressed via inline `// drift-ignore` comments.
pub struct SuppressionChecker;

impl SuppressionChecker {
    pub fn new() -> Self {
        Self
    }

    /// Check if a violation at the given file:line is suppressed.
    ///
    /// Supports:
    /// - `// drift-ignore` — suppress all rules on the next line
    /// - `// drift-ignore security/sql-injection` — suppress specific rule
    /// - `// drift-ignore security/sql-injection, naming/camelCase` — suppress multiple rules
    pub fn is_suppressed(
        &self,
        file: &str,
        line: u32,
        rule_id: Option<&str>,
        source_lines: &HashMap<String, Vec<String>>,
    ) -> bool {
        let lines = match source_lines.get(file) {
            Some(l) => l,
            None => return false,
        };

        // Check the line above the violation for drift-ignore
        if line == 0 {
            return false;
        }
        let check_line = (line - 1) as usize;
        if check_line == 0 || check_line > lines.len() {
            return false;
        }

        // Check the line immediately above
        let prev_line = &lines[check_line.saturating_sub(1)];
        self.line_suppresses(prev_line, rule_id)
    }

    /// Parse a line for drift-ignore directives.
    fn line_suppresses(&self, line: &str, rule_id: Option<&str>) -> bool {
        let trimmed = line.trim();

        // Find drift-ignore in the line (could be after code)
        let ignore_marker = "drift-ignore";
        let pos = match trimmed.find(ignore_marker) {
            Some(p) => p,
            None => return false,
        };

        // Verify it's in a comment context
        let before = &trimmed[..pos];
        let is_comment = before.contains("//")
            || before.contains('#')
            || before.contains("--")
            || before.contains("/*");
        if !is_comment {
            return false;
        }

        // Extract the rest after "drift-ignore"
        let after = trimmed[pos + ignore_marker.len()..].trim();

        // If no specific rules listed, suppress everything
        if after.is_empty() || after.starts_with("--") {
            return true;
        }

        // If a specific rule_id is provided, check if it's in the list
        match rule_id {
            None => true,
            Some(rid) => {
                let rules: Vec<&str> = after.split(',').map(|s| s.trim()).collect();
                rules.contains(&rid)
            }
        }
    }

    /// Extract all suppression directives from source lines.
    pub fn extract_suppressions(
        &self,
        file: &str,
        lines: &[String],
    ) -> Vec<SuppressionDirective> {
        let mut directives = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if let Some(directive) = self.parse_directive(file, i as u32 + 1, line) {
                directives.push(directive);
            }
        }
        directives
    }

    fn parse_directive(
        &self,
        file: &str,
        line_num: u32,
        line: &str,
    ) -> Option<SuppressionDirective> {
        let trimmed = line.trim();
        let marker = "drift-ignore";
        let pos = trimmed.find(marker)?;

        let before = &trimmed[..pos];
        let is_comment = before.contains("//")
            || before.contains('#')
            || before.contains("--")
            || before.contains("/*");
        if !is_comment {
            return None;
        }

        let after = trimmed[pos + marker.len()..].trim();
        let rule_ids = if after.is_empty() {
            Vec::new()
        } else {
            after.split(',').map(|s| s.trim().to_string()).collect()
        };

        Some(SuppressionDirective {
            file: file.to_string(),
            line: line_num,
            applies_to_line: line_num + 1,
            rule_ids,
        })
    }
}

impl Default for SuppressionChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// A parsed suppression directive.
#[derive(Debug, Clone)]
pub struct SuppressionDirective {
    pub file: String,
    pub line: u32,
    pub applies_to_line: u32,
    pub rule_ids: Vec<String>,
}
