//! Bayesian convention discovery.
//!
//! Discovers conventions from aggregated + scored patterns.
//! Thresholds: minOccurrences=3, dominance=0.60, minFiles=2.

use crate::patterns::aggregation::types::AggregatedPattern;
use crate::patterns::confidence::types::{ConfidenceScore, ConfidenceTier, MomentumDirection};

use super::types::{Convention, ConventionCategory, ConventionScope, LearningConfig, PromotionStatus};

/// Discovers conventions from aggregated and scored patterns.
pub struct ConventionDiscoverer {
    config: LearningConfig,
}

impl ConventionDiscoverer {
    /// Create a new discoverer with default configuration.
    pub fn new() -> Self {
        Self {
            config: LearningConfig::default(),
        }
    }

    /// Create a discoverer with custom configuration.
    pub fn with_config(config: LearningConfig) -> Self {
        Self { config }
    }

    /// Discover conventions from aggregated patterns with their confidence scores.
    ///
    /// `patterns`: aggregated patterns from the aggregation pipeline.
    /// `scores`: confidence scores keyed by pattern_id.
    /// `total_files`: total files in the project.
    /// `now`: current unix timestamp.
    pub fn discover(
        &self,
        patterns: &[AggregatedPattern],
        scores: &[(String, ConfidenceScore)],
        total_files: u64,
        now: u64,
    ) -> Vec<Convention> {
        let score_map: std::collections::HashMap<&str, &ConfidenceScore> = scores
            .iter()
            .map(|(id, s)| (id.as_str(), s))
            .collect();

        let mut conventions = Vec::new();

        // Group patterns by category to detect contested conventions
        let mut category_groups: std::collections::HashMap<String, Vec<&AggregatedPattern>> =
            std::collections::HashMap::new();
        for pattern in patterns {
            category_groups
                .entry(pattern.category.name().to_string())
                .or_default()
                .push(pattern);
        }

        for pattern in patterns {
            // Check minimum thresholds
            if (pattern.location_count as u64) < self.config.min_occurrences {
                continue;
            }
            if (pattern.file_spread as u64) < self.config.min_files {
                continue;
            }

            // Compute dominance ratio within category
            let category_total: u64 = category_groups
                .get(pattern.category.name())
                .map(|group| group.iter().map(|p| p.location_count as u64).sum())
                .unwrap_or(0);

            let dominance = if category_total > 0 {
                pattern.location_count as f64 / category_total as f64
            } else {
                0.0
            };

            if dominance < self.config.dominance_threshold {
                // Check if this is a contested convention
                let is_contested = self.check_contested(pattern, &category_groups);
                if !is_contested {
                    continue;
                }
            }

            // Get confidence score
            let score = score_map
                .get(pattern.pattern_id.as_str())
                .cloned()
                .cloned()
                .unwrap_or_else(ConfidenceScore::uniform_prior);

            // Classify category
            let spread_ratio = if total_files > 0 {
                pattern.file_spread as f64 / total_files as f64
            } else {
                0.0
            };

            let category = self.classify_category(
                spread_ratio,
                &score,
                dominance,
                pattern,
                &category_groups,
            );

            conventions.push(Convention {
                id: format!("conv_{}", pattern.pattern_id),
                pattern_id: pattern.pattern_id.clone(),
                category,
                scope: ConventionScope::Project,
                confidence_score: score,
                dominance_ratio: dominance,
                discovery_date: now,
                last_seen: now,
                promotion_status: PromotionStatus::Discovered,
            });
        }

        conventions
    }

    /// Classify a convention into one of 5 categories.
    fn classify_category(
        &self,
        spread_ratio: f64,
        score: &ConfidenceScore,
        dominance: f64,
        pattern: &AggregatedPattern,
        category_groups: &std::collections::HashMap<String, Vec<&AggregatedPattern>>,
    ) -> ConventionCategory {
        // Check contested first
        if self.check_contested(pattern, category_groups) {
            return ConventionCategory::Contested;
        }

        // Universal: high spread + established confidence
        if spread_ratio >= self.config.universal_spread_threshold
            && score.tier == ConfidenceTier::Established
        {
            return ConventionCategory::Universal;
        }

        // Emerging: rising momentum
        if score.momentum == MomentumDirection::Rising {
            return ConventionCategory::Emerging;
        }

        // Legacy: falling momentum
        if score.momentum == MomentumDirection::Falling {
            return ConventionCategory::Legacy;
        }

        // Default: project-specific
        ConventionCategory::ProjectSpecific
    }

    /// Check if a pattern is part of a contested convention.
    ///
    /// Two patterns within 15% frequency of each other → contested.
    fn check_contested(
        &self,
        pattern: &AggregatedPattern,
        category_groups: &std::collections::HashMap<String, Vec<&AggregatedPattern>>,
    ) -> bool {
        let group = match category_groups.get(pattern.category.name()) {
            Some(g) => g,
            None => return false,
        };

        let total: u64 = group.iter().map(|p| p.location_count as u64).sum();
        if total == 0 {
            return false;
        }

        let my_ratio = pattern.location_count as f64 / total as f64;

        for other in group {
            if other.pattern_id == pattern.pattern_id {
                continue;
            }
            let other_ratio = other.location_count as f64 / total as f64;
            if (my_ratio - other_ratio).abs() <= self.config.contested_threshold {
                return true;
            }
        }

        false
    }
}

impl Default for ConventionDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::PatternCategory;
    use crate::patterns::aggregation::types::PatternLocation;

    fn make_pattern(id: &str, locations: u32, files: u32) -> AggregatedPattern {
        let locs: Vec<PatternLocation> = (0..locations)
            .map(|i| PatternLocation {
                file: format!("file_{}.ts", i % files),
                line: i + 1,
                column: 0,
                confidence: 0.9,
                is_outlier: false,
                matched_text: None,
            })
            .collect();
        AggregatedPattern {
            pattern_id: id.to_string(),
            category: PatternCategory::Structural,
            location_count: locations,
            outlier_count: 0,
            file_spread: files,
            hierarchy: None,
            locations: locs,
            aliases: Vec::new(),
            merged_from: Vec::new(),
            confidence_mean: 0.9,
            confidence_stddev: 0.05,
            confidence_values: vec![0.9; locations as usize],
            is_dirty: false,
            location_hash: 0,
        }
    }

    #[test]
    fn test_discover_basic_convention() {
        let discoverer = ConventionDiscoverer::new();
        let patterns = vec![make_pattern("dominant", 80, 10)];
        let scores = vec![(
            "dominant".to_string(),
            ConfidenceScore::from_params(90.0, 10.0, MomentumDirection::Stable),
        )];

        let conventions = discoverer.discover(&patterns, &scores, 100, 1000);
        assert_eq!(conventions.len(), 1);
        assert_eq!(conventions[0].pattern_id, "dominant");
    }

    #[test]
    fn test_below_threshold_not_discovered() {
        let discoverer = ConventionDiscoverer::new();
        let patterns = vec![make_pattern("rare", 2, 1)]; // Below min_occurrences and min_files
        let scores = vec![(
            "rare".to_string(),
            ConfidenceScore::uniform_prior(),
        )];

        let conventions = discoverer.discover(&patterns, &scores, 100, 1000);
        assert!(conventions.is_empty());
    }

    #[test]
    fn test_contested_convention() {
        let discoverer = ConventionDiscoverer::new();
        let patterns = vec![
            make_pattern("style_a", 45, 10),
            make_pattern("style_b", 55, 12),
        ];
        let scores = vec![
            ("style_a".to_string(), ConfidenceScore::from_params(10.0, 5.0, MomentumDirection::Stable)),
            ("style_b".to_string(), ConfidenceScore::from_params(12.0, 5.0, MomentumDirection::Stable)),
        ];

        let conventions = discoverer.discover(&patterns, &scores, 100, 1000);
        let contested: Vec<_> = conventions
            .iter()
            .filter(|c| c.category == ConventionCategory::Contested)
            .collect();
        assert!(!contested.is_empty(), "Should detect contested convention");
    }
}
