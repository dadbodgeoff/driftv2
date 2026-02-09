//! Top-level ConfidenceScorer — takes aggregated patterns, computes Beta posteriors,
//! assigns tiers, tracks momentum.

use crate::patterns::aggregation::types::AggregatedPattern;

use super::beta::BetaPosterior;
use super::factors::{self, FactorInput};
use super::momentum::{self, MomentumTracker};
use super::types::{ConfidenceScore, ConfidenceTier, MomentumDirection};

/// Configuration for the confidence scorer.
#[derive(Debug, Clone)]
pub struct ScorerConfig {
    /// Total files in the project (for spread calculation).
    pub total_files: u64,
    /// Default days since first seen (when unknown).
    pub default_age_days: u64,
}

impl Default for ScorerConfig {
    fn default() -> Self {
        Self {
            total_files: 100,
            default_age_days: 7,
        }
    }
}

/// The top-level confidence scorer.
///
/// Takes aggregated patterns and produces ConfidenceScore for each.
pub struct ConfidenceScorer {
    config: ScorerConfig,
}

impl ConfidenceScorer {
    /// Create a new scorer with the given configuration.
    pub fn new(config: ScorerConfig) -> Self {
        Self { config }
    }

    /// Create a scorer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ScorerConfig::default())
    }

    /// Score a single aggregated pattern.
    ///
    /// Combines Beta distribution posterior with 5-factor model.
    pub fn score(
        &self,
        pattern: &AggregatedPattern,
        momentum: MomentumDirection,
        days_since_first_seen: u64,
    ) -> ConfidenceScore {
        // Step 1: Compute raw Beta posterior from observation counts
        let total_observations = self.config.total_files;
        let successes = pattern.file_spread as u64;
        let (base_alpha, base_beta) = BetaPosterior::posterior_params(successes, total_observations);

        // Step 2: Compute 5-factor adjustments
        let factor_input = FactorInput {
            occurrences: pattern.location_count as u64,
            total_locations: total_observations.max(1),
            variance: pattern.confidence_stddev.powi(2),
            days_since_first_seen,
            file_count: pattern.file_spread as u64,
            total_files: self.config.total_files,
            momentum,
        };

        let factor_values = factors::compute_factors(&factor_input);
        let (alpha_adj, beta_adj) = factors::factors_to_alpha_beta(
            &factor_values,
            pattern.location_count as u64,
        );

        // Step 3: Combine base posterior with factor adjustments
        let final_alpha = base_alpha + alpha_adj;
        let final_beta = base_beta + beta_adj;

        // Step 4: Build the score
        ConfidenceScore::from_params(final_alpha, final_beta, momentum)
    }

    /// Score all patterns in a batch.
    pub fn score_batch(
        &self,
        patterns: &[AggregatedPattern],
    ) -> Vec<(String, ConfidenceScore)> {
        patterns
            .iter()
            .map(|p| {
                let score = self.score(p, MomentumDirection::Stable, self.config.default_age_days);
                (p.pattern_id.clone(), score)
            })
            .collect()
    }

    /// Score with full context including momentum tracker.
    pub fn score_with_momentum(
        &self,
        pattern: &AggregatedPattern,
        tracker: &MomentumTracker,
        days_since_first_seen: u64,
        days_since_last_seen: u64,
    ) -> ConfidenceScore {
        let momentum = tracker.direction();
        let mut score = self.score(pattern, momentum, days_since_first_seen);

        // Apply temporal decay if pattern hasn't been seen recently
        let decay = momentum::temporal_decay(days_since_last_seen);
        if decay < 1.0 {
            score.alpha *= decay;
            // Recompute derived values
            score.posterior_mean = BetaPosterior::posterior_mean(score.alpha, score.beta);
            score.tier = ConfidenceTier::from_posterior_mean(score.posterior_mean);
            score.credible_interval = super::beta::credible_interval(score.alpha, score.beta, 0.95);
        }

        score.momentum = momentum;
        score
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
    fn test_score_high_spread_pattern() {
        let scorer = ConfidenceScorer::new(ScorerConfig {
            total_files: 100,
            default_age_days: 30,
        });
        let pattern = make_pattern("test", 95, 95);
        let score = scorer.score(&pattern, MomentumDirection::Rising, 30);
        assert_eq!(score.tier, ConfidenceTier::Established);
        assert!(score.posterior_mean >= 0.85);
    }

    #[test]
    fn test_score_low_spread_pattern() {
        let scorer = ConfidenceScorer::new(ScorerConfig {
            total_files: 100,
            default_age_days: 7,
        });
        let pattern = make_pattern("test", 3, 2);
        let score = scorer.score(&pattern, MomentumDirection::Stable, 1);
        assert!(score.tier != ConfidenceTier::Established);
    }

    #[test]
    fn test_score_batch() {
        let scorer = ConfidenceScorer::with_defaults();
        let patterns = vec![
            make_pattern("a", 50, 40),
            make_pattern("b", 10, 5),
        ];
        let scores = scorer.score_batch(&patterns);
        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0].0, "a");
        assert_eq!(scores[1].0, "b");
    }

    #[test]
    fn test_temporal_decay_drops_tier() {
        let scorer = ConfidenceScorer::new(ScorerConfig {
            total_files: 100,
            default_age_days: 30,
        });
        let pattern = make_pattern("test", 90, 85);
        let mut tracker = MomentumTracker::new();
        for _ in 0..5 {
            tracker.record(90);
        }

        let fresh = scorer.score_with_momentum(&pattern, &tracker, 30, 0);
        let stale = scorer.score_with_momentum(&pattern, &tracker, 30, 60);

        assert!(
            stale.posterior_mean < fresh.posterior_mean,
            "Stale pattern should have lower confidence"
        );
    }
}
