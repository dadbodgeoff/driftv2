//! 5-factor model: Frequency, Consistency, Age, Spread, Momentum.
//!
//! Each factor contributes to alpha/beta updates on the Beta distribution.
//! Weights: frequency=0.30, consistency=0.25, age=0.10, spread=0.15, momentum=0.20.

use super::types::MomentumDirection;

/// Weights for the 5-factor model.
pub const WEIGHT_FREQUENCY: f64 = 0.30;
pub const WEIGHT_CONSISTENCY: f64 = 0.25;
pub const WEIGHT_AGE: f64 = 0.10;
pub const WEIGHT_SPREAD: f64 = 0.15;
pub const WEIGHT_MOMENTUM: f64 = 0.20;

/// Input data for the 5-factor model.
#[derive(Debug, Clone)]
pub struct FactorInput {
    /// Number of pattern occurrences.
    pub occurrences: u64,
    /// Total applicable locations (files × applicable sites).
    pub total_locations: u64,
    /// Confidence variance across locations (0 = perfectly consistent).
    pub variance: f64,
    /// Days since first seen.
    pub days_since_first_seen: u64,
    /// Number of files containing the pattern.
    pub file_count: u64,
    /// Total files in scope.
    pub total_files: u64,
    /// Momentum direction.
    pub momentum: MomentumDirection,
}

/// Computed factor values (each normalized to [0.0, 1.0]).
#[derive(Debug, Clone)]
pub struct FactorValues {
    pub frequency: f64,
    pub consistency: f64,
    pub age: f64,
    pub spread: f64,
    pub momentum: f64,
}

/// Compute all 5 factors from input data.
pub fn compute_factors(input: &FactorInput) -> FactorValues {
    FactorValues {
        frequency: compute_frequency(input.occurrences, input.total_locations),
        consistency: compute_consistency(input.variance),
        age: compute_age(input.days_since_first_seen),
        spread: compute_spread(input.file_count, input.total_files),
        momentum: compute_momentum(input.momentum),
    }
}

/// Compute the weighted composite score from factor values.
pub fn weighted_score(factors: &FactorValues) -> f64 {
    let score = factors.frequency * WEIGHT_FREQUENCY
        + factors.consistency * WEIGHT_CONSISTENCY
        + factors.age * WEIGHT_AGE
        + factors.spread * WEIGHT_SPREAD
        + factors.momentum * WEIGHT_MOMENTUM;
    score.clamp(0.0, 1.0)
}

/// Convert factor values into alpha/beta adjustments for the Beta distribution.
///
/// The weighted score determines how much evidence to add:
/// - High score → more alpha (successes)
/// - Low score → more beta (failures)
///
/// `sample_size` controls the strength of the update (more data → stronger update).
pub fn factors_to_alpha_beta(factors: &FactorValues, sample_size: u64) -> (f64, f64) {
    let score = weighted_score(factors);
    let n = (sample_size as f64).max(1.0);

    // Sample-size-adaptive blending: larger samples → stronger Bayesian update
    let blend_weight = (n / (n + 10.0)).min(1.0); // Sigmoid-like ramp

    let alpha_contribution = score * blend_weight * n;
    let beta_contribution = (1.0 - score) * blend_weight * n;

    (alpha_contribution.max(0.0), beta_contribution.max(0.0))
}

/// Factor 1: Frequency — how often the pattern appears.
fn compute_frequency(occurrences: u64, total_locations: u64) -> f64 {
    if total_locations == 0 {
        return 0.0;
    }
    let freq = occurrences as f64 / total_locations as f64;
    freq.clamp(0.0, 1.0)
}

/// Factor 2: Consistency — how uniformly across files (1 - variance).
fn compute_consistency(variance: f64) -> f64 {
    if !variance.is_finite() || variance < 0.0 {
        return 1.0; // Treat invalid variance as perfectly consistent
    }
    (1.0 - variance).clamp(0.0, 1.0)
}

/// Factor 3: Age — how long established (linear ramp over 30 days).
fn compute_age(days_since_first_seen: u64) -> f64 {
    const MIN_AGE_FACTOR: f64 = 0.1;
    const MAX_AGE_DAYS: f64 = 30.0;

    if days_since_first_seen == 0 {
        return MIN_AGE_FACTOR;
    }
    let days = days_since_first_seen as f64;
    if days >= MAX_AGE_DAYS {
        return 1.0;
    }
    let normalized = days / MAX_AGE_DAYS;
    MIN_AGE_FACTOR + normalized * (1.0 - MIN_AGE_FACTOR)
}

/// Factor 4: Spread — how many files contain the pattern.
fn compute_spread(file_count: u64, total_files: u64) -> f64 {
    if total_files == 0 {
        return 0.0;
    }
    let spread = file_count as f64 / total_files as f64;
    spread.clamp(0.0, 1.0)
}

/// Factor 5: Momentum — trend direction.
fn compute_momentum(direction: MomentumDirection) -> f64 {
    match direction {
        MomentumDirection::Rising => 0.8,
        MomentumDirection::Stable => 0.5,
        MomentumDirection::Falling => 0.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_zero_total() {
        assert_eq!(compute_frequency(5, 0), 0.0);
    }

    #[test]
    fn test_frequency_normal() {
        let f = compute_frequency(50, 100);
        assert!((f - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_consistency_zero_variance() {
        assert!((compute_consistency(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_consistency_high_variance() {
        assert!(compute_consistency(0.8) < 0.3);
    }

    #[test]
    fn test_age_brand_new() {
        assert!((compute_age(0) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_age_mature() {
        assert!((compute_age(30) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_spread_zero_files() {
        assert_eq!(compute_spread(0, 0), 0.0);
    }

    #[test]
    fn test_momentum_values() {
        assert!(compute_momentum(MomentumDirection::Rising) > compute_momentum(MomentumDirection::Stable));
        assert!(compute_momentum(MomentumDirection::Stable) > compute_momentum(MomentumDirection::Falling));
    }

    #[test]
    fn test_weighted_score_sums_correctly() {
        let sum = WEIGHT_FREQUENCY + WEIGHT_CONSISTENCY + WEIGHT_AGE + WEIGHT_SPREAD + WEIGHT_MOMENTUM;
        assert!((sum - 1.0).abs() < 1e-10, "Weights must sum to 1.0");
    }
}
