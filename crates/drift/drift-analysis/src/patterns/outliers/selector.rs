//! Auto-select method based on sample size and data characteristics.

use super::types::{OutlierConfig, OutlierMethod, OutlierResult};
use super::{esd, grubbs, iqr, mad, rule_based, zscore};

/// The top-level outlier detector with automatic method selection.
pub struct OutlierDetector {
    config: OutlierConfig,
    rules: Vec<rule_based::OutlierRule>,
}

impl OutlierDetector {
    /// Create a new detector with default configuration.
    pub fn new() -> Self {
        Self {
            config: OutlierConfig::default(),
            rules: vec![rule_based::zero_confidence_rule()],
        }
    }

    /// Create a detector with custom configuration.
    pub fn with_config(config: OutlierConfig) -> Self {
        Self {
            config,
            rules: vec![rule_based::zero_confidence_rule()],
        }
    }

    /// Add a custom rule.
    pub fn add_rule(&mut self, rule: rule_based::OutlierRule) {
        self.rules.push(rule);
    }

    /// Detect outliers with automatic method selection.
    ///
    /// Returns all detected outliers, merged and deduplicated.
    pub fn detect(&self, values: &[f64]) -> Vec<OutlierResult> {
        let n = values.len();

        if n < self.config.min_sample_size {
            // Only rule-based for insufficient data
            return rule_based::detect(values, &self.rules);
        }

        let mut all_results = Vec::new();

        // Primary statistical method based on sample size
        let primary = self.select_primary_method(n);
        let statistical = match primary {
            OutlierMethod::ZScore => {
                zscore::detect(values, self.config.z_threshold, self.config.max_iterations)
            }
            OutlierMethod::Grubbs => grubbs::detect(values, self.config.alpha),
            OutlierMethod::GeneralizedEsd => {
                let max_outliers = (n / 5).clamp(1, 10);
                esd::detect(values, max_outliers, self.config.alpha)
            }
            _ => Vec::new(),
        };
        all_results.extend(statistical);

        // Supplementary IQR for n ≥ 30 (cross-validation)
        if n >= 30 {
            let iqr_results = iqr::detect(values, self.config.iqr_multiplier);
            // Boost significance for outliers found by both methods
            for iqr_r in &iqr_results {
                if !all_results.iter().any(|r| r.index == iqr_r.index) {
                    all_results.push(iqr_r.clone());
                }
            }
        }

        // MAD for robustness check
        let mad_results = mad::detect(values, self.config.mad_threshold);
        for mad_r in &mad_results {
            if !all_results.iter().any(|r| r.index == mad_r.index) {
                all_results.push(mad_r.clone());
            }
        }

        // Rule-based (always active)
        let rule_results = rule_based::detect(values, &self.rules);
        for rule_r in &rule_results {
            if !all_results.iter().any(|r| r.index == rule_r.index) {
                all_results.push(rule_r.clone());
            }
        }

        all_results
    }

    /// Select the primary statistical method based on sample size.
    pub fn select_primary_method(&self, n: usize) -> OutlierMethod {
        if n >= 30 {
            OutlierMethod::ZScore
        } else if n >= 25 {
            OutlierMethod::GeneralizedEsd
        } else if n >= 10 {
            OutlierMethod::Grubbs
        } else {
            OutlierMethod::RuleBased
        }
    }
}

impl Default for OutlierDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_select_zscore() {
        let detector = OutlierDetector::new();
        assert_eq!(detector.select_primary_method(30), OutlierMethod::ZScore);
        assert_eq!(detector.select_primary_method(100), OutlierMethod::ZScore);
    }

    #[test]
    fn test_auto_select_grubbs() {
        let detector = OutlierDetector::new();
        assert_eq!(detector.select_primary_method(10), OutlierMethod::Grubbs);
        assert_eq!(detector.select_primary_method(24), OutlierMethod::Grubbs);
    }

    #[test]
    fn test_auto_select_esd() {
        let detector = OutlierDetector::new();
        assert_eq!(detector.select_primary_method(25), OutlierMethod::GeneralizedEsd);
        assert_eq!(detector.select_primary_method(29), OutlierMethod::GeneralizedEsd);
    }

    #[test]
    fn test_auto_select_rule_based() {
        let detector = OutlierDetector::new();
        assert_eq!(detector.select_primary_method(5), OutlierMethod::RuleBased);
    }

    #[test]
    fn test_detect_with_clear_outlier() {
        let detector = OutlierDetector::new();
        let mut values: Vec<f64> = vec![0.9; 50];
        values[0] = 0.01; // Clear outlier
        let results = detector.detect(&values);
        assert!(!results.is_empty());
    }
}
