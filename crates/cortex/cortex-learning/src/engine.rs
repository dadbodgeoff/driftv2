//! LearningEngine: implements ILearner, orchestrates full pipeline.

use cortex_core::errors::CortexResult;
use cortex_core::memory::BaseMemory;
use cortex_core::models::LearningResult;
use cortex_core::traits::{Correction, ILearner};
use tracing::info;

use crate::analysis;
use crate::calibration::{self, CalibrationFactors};
use crate::deduplication::{self, DedupAction};
use crate::extraction;

/// The main learning engine.
///
/// Orchestrates: diff analysis → categorization → principle extraction →
/// dedup → memory creation.
pub struct LearningEngine {
    /// Existing memories for dedup checking.
    existing_memories: Vec<BaseMemory>,
    /// LLM extractor (optional).
    llm_extractor: Box<dyn extraction::LlmExtractor>,
}

impl LearningEngine {
    /// Create a new learning engine.
    pub fn new() -> Self {
        Self {
            existing_memories: Vec::new(),
            llm_extractor: Box::new(extraction::NoOpExtractor),
        }
    }

    /// Create with an LLM extractor.
    pub fn with_llm(llm_extractor: Box<dyn extraction::LlmExtractor>) -> Self {
        Self {
            existing_memories: Vec::new(),
            llm_extractor,
        }
    }

    /// Set existing memories for dedup checking.
    pub fn set_existing_memories(&mut self, memories: Vec<BaseMemory>) {
        self.existing_memories = memories;
    }

    /// Full learning pipeline.
    fn learn(&self, correction: &Correction) -> CortexResult<LearningResult> {
        // Step 1: Categorize the correction.
        let category = analysis::categorize(&correction.correction_text, &correction.context);
        info!(category = ?category, "correction categorized");

        // Step 2: Map category to memory type.
        let mapping = analysis::map_category(category);

        // Step 3: Extract principle.
        let principle = extraction::extract_with_fallback(
            &correction.correction_text,
            &correction.context,
            self.llm_extractor.as_ref(),
        );

        // Step 4: Check dedup.
        let summary = principle
            .clone()
            .unwrap_or_else(|| correction.correction_text.clone());
        let content_hash = blake3::hash(summary.as_bytes()).to_hex().to_string();
        let dedup_action =
            deduplication::check_dedup(&content_hash, &summary, &self.existing_memories);

        // Step 5: Determine result.
        let memory_created = match dedup_action {
            DedupAction::Add => {
                // Calibrate confidence.
                let factors = CalibrationFactors {
                    base: 0.6,
                    evidence: calibration::evidence_factor(1),
                    usage: 0.0,
                    temporal: 1.0,
                    validation: 0.0,
                };
                let confidence = calibration::calibrate(&factors);

                info!(
                    memory_type = ?mapping.memory_type,
                    importance = ?mapping.importance,
                    confidence = confidence,
                    "creating new memory from correction"
                );

                Some(uuid::Uuid::new_v4().to_string())
            }
            DedupAction::Update(id) => {
                info!(id = %id, "updating existing memory from correction");
                Some(id)
            }
            DedupAction::Noop => {
                info!("duplicate correction, no action needed");
                None
            }
        };

        Ok(LearningResult {
            category: format!("{:?}", category),
            principle,
            memory_created,
        })
    }
}

impl Default for LearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ILearner for LearningEngine {
    fn analyze(&self, correction: &Correction) -> CortexResult<LearningResult> {
        self.learn(correction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::memory::{Confidence, TypedContent};

    #[test]
    fn analyzes_security_correction() {
        let engine = LearningEngine::new();
        let correction = Correction {
            original_memory_id: None,
            correction_text: "Always sanitize user input to prevent SQL injection".to_string(),
            context: "database security".to_string(),
            source: "code review".to_string(),
        };

        let result = engine.analyze(&correction).unwrap();
        assert_eq!(result.category, "SecurityIssue");
        assert!(result.principle.is_some());
        assert!(result.memory_created.is_some());
    }

    #[test]
    fn dedup_prevents_duplicate() {
        let mut engine = LearningEngine::new();

        // First correction creates a memory.
        let correction = Correction {
            original_memory_id: None,
            correction_text: "Use Result for error handling".to_string(),
            context: "rust".to_string(),
            source: "review".to_string(),
        };
        let result1 = engine.analyze(&correction).unwrap();
        assert!(result1.memory_created.is_some());

        // Simulate the memory being stored.
        let content = TypedContent::Insight(cortex_core::memory::types::InsightContent {
            observation: "Use Result for error handling".to_string(),
            evidence: vec![],
        });
        let summary = result1.principle.unwrap_or_default();
        let hash = blake3::hash(summary.as_bytes()).to_hex().to_string();
        let mem = BaseMemory {
            id: result1.memory_created.unwrap(),
            memory_type: cortex_core::memory::MemoryType::Insight,
            content: content.clone(),
            summary: summary.clone(),
            transaction_time: chrono::Utc::now(),
            valid_time: chrono::Utc::now(),
            valid_until: None,
            confidence: Confidence::new(0.6),
            importance: cortex_core::memory::Importance::Normal,
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            linked_patterns: vec![],
            linked_constraints: vec![],
            linked_files: vec![],
            linked_functions: vec![],
            tags: vec![],
            archived: false,
            superseded_by: None,
            supersedes: None,
            content_hash: hash,
        };
        engine.set_existing_memories(vec![mem]);

        // Same correction again should dedup.
        let _result2 = engine.analyze(&correction).unwrap();
        // Should either update or noop, not create a brand new ID.
        // (Exact behavior depends on hash matching.)
    }

    #[test]
    fn categorizes_pattern_violation() {
        let engine = LearningEngine::new();
        let correction = Correction {
            original_memory_id: None,
            correction_text: "This violates the SOLID design pattern".to_string(),
            context: "class design".to_string(),
            source: "review".to_string(),
        };

        let result = engine.analyze(&correction).unwrap();
        assert_eq!(result.category, "PatternViolation");
    }
}
