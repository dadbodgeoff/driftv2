//! Zone classification and trend tracking.

use super::types::{CouplingMetrics, CouplingTrend, TrendDirection, ZoneClassification};

/// Distance threshold for main sequence classification.
const MAIN_SEQUENCE_THRESHOLD: f64 = 0.3;

/// Classify a module into a zone based on instability and abstractness.
///
/// - Zone of Pain: high stability (low I) + low abstractness (low A) → concrete and rigid
/// - Zone of Uselessness: high instability (high I) + high abstractness (high A) → abstract but unused
/// - Main Sequence: |A + I - 1| ≤ threshold → balanced
pub fn classify_zone(instability: f64, abstractness: f64) -> ZoneClassification {
    let distance = (abstractness + instability - 1.0).abs();

    if distance <= MAIN_SEQUENCE_THRESHOLD {
        ZoneClassification::MainSequence
    } else if instability < 0.5 && abstractness < 0.5 {
        ZoneClassification::ZoneOfPain
    } else if instability > 0.5 && abstractness > 0.5 {
        ZoneClassification::ZoneOfUselessness
    } else {
        // Near the edges but not clearly in a zone — default to main sequence
        ZoneClassification::MainSequence
    }
}

/// Compute trend direction between two metric snapshots.
pub fn compute_trend(previous: &CouplingMetrics, current: &CouplingMetrics) -> CouplingTrend {
    let direction = if current.distance < previous.distance - 0.05 {
        TrendDirection::Improving
    } else if current.distance > previous.distance + 0.05 {
        TrendDirection::Degrading
    } else {
        TrendDirection::Stable
    };

    CouplingTrend {
        module: current.module.clone(),
        previous: previous.clone(),
        current: current.clone(),
        direction,
    }
}
