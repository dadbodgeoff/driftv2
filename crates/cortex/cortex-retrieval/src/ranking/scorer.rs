//! Multi-factor relevance scorer (8 factors).
//!
//! Factors: semantic similarity, keyword match, file proximity, pattern alignment,
//! recency, confidence, importance, intent-type match.

use chrono::Utc;

use cortex_core::intent::Intent;
use cortex_core::memory::BaseMemory;

use crate::intent::IntentEngine;
use crate::search::rrf_fusion::RrfCandidate;

/// Weights for the 8 scoring factors.
#[derive(Debug, Clone)]
pub struct ScorerWeights {
    pub semantic_similarity: f64,
    pub keyword_match: f64,
    pub file_proximity: f64,
    pub pattern_alignment: f64,
    pub recency: f64,
    pub confidence: f64,
    pub importance: f64,
    pub intent_type_match: f64,
}

impl Default for ScorerWeights {
    fn default() -> Self {
        Self {
            semantic_similarity: 0.25,
            keyword_match: 0.10,
            file_proximity: 0.10,
            pattern_alignment: 0.10,
            recency: 0.10,
            confidence: 0.10,
            importance: 0.15,
            intent_type_match: 0.10,
        }
    }
}

/// Scored candidate after multi-factor ranking.
#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub memory: BaseMemory,
    /// Final composite score [0.0, ~2.0+].
    pub score: f64,
    /// RRF score from fusion stage.
    pub rrf_score: f64,
}

/// Score a list of RRF candidates using 8 factors.
pub fn score(
    candidates: &[RrfCandidate],
    intent: Intent,
    active_files: &[String],
    intent_engine: &IntentEngine,
    weights: &ScorerWeights,
) -> Vec<ScoredCandidate> {
    let now = Utc::now();
    let max_rrf = candidates
        .first()
        .map(|c| c.rrf_score)
        .unwrap_or(1.0)
        .max(f64::EPSILON);

    let mut scored: Vec<ScoredCandidate> = candidates
        .iter()
        .map(|c| {
            let m = &c.memory;

            // Factor 1: Semantic similarity (normalized RRF as proxy).
            let f_semantic = c.rrf_score / max_rrf;

            // Factor 2: Keyword match — real BM25 rank from FTS5 search.
            // Uses RRF-style normalization: 1/(1 + rank) to produce a smooth signal
            // that doesn't create extreme spread with few candidates.
            let f_keyword = match c.fts5_rank {
                Some(rank) => 1.0 / (1.0 + rank as f64),
                None => 0.0,
            };

            // Factor 3: File proximity — do any linked files match active files?
            let f_file = file_proximity_score(m, active_files);

            // Factor 4: Pattern alignment — has linked patterns?
            let f_pattern = if m.linked_patterns.is_empty() {
                0.0
            } else {
                (m.linked_patterns.len() as f64).min(3.0) / 3.0
            };

            // Factor 5: Recency — exponential decay over days.
            let days_since = (now - m.last_accessed).num_days().max(0) as f64;
            let f_recency = (-days_since / 90.0).exp(); // 90-day half-life.

            // Factor 6: Confidence.
            let f_confidence = m.confidence.value();

            // Factor 7: Importance weight.
            let f_importance = m.importance.weight() / 2.0; // Normalize: Critical=1.0.

            // Factor 8: Intent-type match boost.
            let f_intent = intent_engine.boost(intent, m.memory_type) / 2.0; // Normalize.

            let score = weights.semantic_similarity * f_semantic
                + weights.keyword_match * f_keyword
                + weights.file_proximity * f_file
                + weights.pattern_alignment * f_pattern
                + weights.recency * f_recency
                + weights.confidence * f_confidence
                + weights.importance * f_importance
                + weights.intent_type_match * f_intent;

            ScoredCandidate {
                memory: m.clone(),
                score,
                rrf_score: c.rrf_score,
            }
        })
        .collect();

    // Sort by score descending.
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    scored
}

/// Compute file proximity score [0.0, 1.0].
fn file_proximity_score(memory: &BaseMemory, active_files: &[String]) -> f64 {
    if active_files.is_empty() || memory.linked_files.is_empty() {
        return 0.0;
    }

    let matches = memory
        .linked_files
        .iter()
        .filter(|f| {
            active_files
                .iter()
                .any(|af| af.contains(&f.file_path) || f.file_path.contains(af.as_str()))
        })
        .count();

    (matches as f64 / memory.linked_files.len() as f64).min(1.0)
}
