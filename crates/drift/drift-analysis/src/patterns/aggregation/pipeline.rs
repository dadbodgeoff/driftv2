//! Top-level 7-phase aggregation pipeline orchestrator.

use drift_core::types::collections::{FxHashMap, FxHashSet};

use crate::engine::types::PatternMatch;

use super::gold_layer::{self, GoldLayerResult};
use super::grouper::PatternGrouper;
use super::hierarchy;
use super::incremental;
use super::reconciliation;
use super::similarity::{self, location_key_set, MinHashIndex};
use super::types::{AggregatedPattern, AggregationConfig, MergeCandidate, MergeDecision};

/// The 7-phase aggregation pipeline.
pub struct AggregationPipeline {
    config: AggregationConfig,
}

impl AggregationPipeline {
    /// Create a new pipeline with the given configuration.
    pub fn new(config: AggregationConfig) -> Self {
        Self { config }
    }

    /// Create a pipeline with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AggregationConfig::default())
    }

    /// Run the full 7-phase aggregation pipeline.
    ///
    /// Input: flat list of PatternMatch from all files.
    /// Output: list of AggregatedPattern ready for downstream consumption.
    pub fn run(&self, matches: &[PatternMatch]) -> AggregationResult {
        // Phase 1-2: Group by pattern ID + cross-file merging + dedup
        let mut grouped = PatternGrouper::group(matches);

        // Phase 3-4: Near-duplicate detection
        let patterns_vec: Vec<&AggregatedPattern> = grouped.values().collect();
        let candidates = self.detect_duplicates(&patterns_vec);

        // Phase 5: Hierarchy building (merge auto-merge candidates)
        hierarchy::build_hierarchies(&mut grouped, &candidates);

        // Phase 6: Counter reconciliation
        for pattern in grouped.values_mut() {
            reconciliation::reconcile(pattern);
        }

        // Phase 7: Gold layer refresh
        let all_patterns: Vec<AggregatedPattern> = grouped.into_values().collect();
        let gold = gold_layer::prepare_gold_layer(&all_patterns);

        AggregationResult {
            patterns: all_patterns,
            merge_candidates: candidates,
            gold_layer: gold,
        }
    }

    /// Run incremental aggregation — only re-aggregate changed files.
    pub fn run_incremental(
        &self,
        matches: &[PatternMatch],
        existing_patterns: &mut Vec<AggregatedPattern>,
        changed_files: &FxHashSet<String>,
    ) -> AggregationResult {
        // Filter to only changed file matches
        let changed_matches: Vec<PatternMatch> = matches
            .iter()
            .filter(|m| changed_files.contains(&m.file))
            .cloned()
            .collect();

        // Remove stale locations from existing patterns
        let affected_ids = incremental::patterns_needing_reaggregation(existing_patterns, changed_files);
        for pattern in existing_patterns.iter_mut() {
            if affected_ids.contains(&pattern.pattern_id) {
                incremental::remove_stale_locations(pattern, changed_files);
            }
        }

        // Group the new matches
        let new_grouped = PatternGrouper::group(&changed_matches);

        // Merge new data into existing patterns
        let mut all_patterns: FxHashMap<String, AggregatedPattern> = existing_patterns
            .drain(..)
            .map(|p| (p.pattern_id.clone(), p))
            .collect();

        for (id, new_pattern) in new_grouped {
            if let Some(existing) = all_patterns.get_mut(&id) {
                existing.locations.extend(new_pattern.locations);
                existing.is_dirty = true;
            } else {
                all_patterns.insert(id, new_pattern);
            }
        }

        // Reconcile all affected patterns
        for pattern in all_patterns.values_mut() {
            reconciliation::reconcile(pattern);
        }

        let patterns: Vec<AggregatedPattern> = all_patterns.into_values().collect();
        let gold = gold_layer::prepare_gold_layer(&patterns);

        AggregationResult {
            patterns,
            merge_candidates: Vec::new(),
            gold_layer: gold,
        }
    }

    /// Phase 3-4: Detect near-duplicate patterns.
    fn detect_duplicates(&self, patterns: &[&AggregatedPattern]) -> Vec<MergeCandidate> {
        let n = patterns.len();
        let use_minhash = self.config.minhash_enabled
            || (n > self.config.minhash_auto_threshold);

        if use_minhash {
            self.detect_duplicates_minhash(patterns)
        } else {
            similarity::find_duplicates(
                patterns,
                self.config.duplicate_flag_threshold,
                self.config.auto_merge_threshold,
            )
        }
    }

    /// MinHash LSH-based duplicate detection for large pattern sets.
    fn detect_duplicates_minhash(&self, patterns: &[&AggregatedPattern]) -> Vec<MergeCandidate> {
        let mut index = MinHashIndex::new(self.config.minhash_num_perm, self.config.minhash_num_bands);

        // Build index
        for pattern in patterns {
            let key_set = location_key_set(pattern);
            index.insert(&pattern.pattern_id, &key_set);
        }

        // Find candidates and verify with estimated similarity
        let raw_candidates = index.find_candidates();
        let mut candidates = Vec::new();

        for (id_a, id_b) in raw_candidates {
            if let Some(sim) = index.estimate_similarity(&id_a, &id_b) {
                if sim >= self.config.duplicate_flag_threshold {
                    let decision = MergeDecision::from_similarity(sim);
                    candidates.push(MergeCandidate {
                        pattern_a: id_a,
                        pattern_b: id_b,
                        similarity: sim,
                        decision,
                    });
                }
            }
        }

        candidates
    }
}

/// Result of the aggregation pipeline.
#[derive(Debug)]
pub struct AggregationResult {
    /// All aggregated patterns (including merged children for audit trail).
    pub patterns: Vec<AggregatedPattern>,
    /// Merge candidates detected during similarity analysis.
    pub merge_candidates: Vec<MergeCandidate>,
    /// Gold layer output ready for persistence.
    pub gold_layer: GoldLayerResult,
}

impl AggregationResult {
    /// Get only the top-level patterns (excluding merged children).
    pub fn top_level_patterns(&self) -> Vec<&AggregatedPattern> {
        self.patterns
            .iter()
            .filter(|p| {
                p.hierarchy
                    .as_ref()
                    .map(|h| h.parent_id.is_none())
                    .unwrap_or(true)
            })
            .collect()
    }
}
