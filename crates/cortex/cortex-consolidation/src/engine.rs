//! ConsolidationEngine: implements IConsolidator, Arc<AtomicBool> is_running guard.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cortex_core::errors::{ConsolidationError, CortexError, CortexResult};
use cortex_core::memory::BaseMemory;
use cortex_core::models::ConsolidationResult;
use cortex_core::traits::{IConsolidator, IEmbeddingProvider};
use tracing::info;

use crate::monitoring::{self, ConsolidationDashboard, TunableThresholds};
use crate::pipeline;

/// The main consolidation engine.
///
/// Coordinates the 6-phase pipeline, enforces single-execution guard,
/// tracks quality metrics, and manages auto-tuning.
pub struct ConsolidationEngine {
    /// Guard: only one consolidation can run at a time.
    is_running: Arc<AtomicBool>,
    /// Embedding provider for similarity computations.
    embedding_provider: Box<dyn IEmbeddingProvider>,
    /// Quality monitoring dashboard.
    dashboard: ConsolidationDashboard,
    /// Tunable thresholds (adjusted by auto-tuning).
    thresholds: TunableThresholds,
    /// Recent quality assessments for auto-tuning.
    recent_assessments: Vec<monitoring::QualityAssessment>,
}

impl ConsolidationEngine {
    /// Create a new consolidation engine.
    pub fn new(embedding_provider: Box<dyn IEmbeddingProvider>) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            embedding_provider,
            dashboard: ConsolidationDashboard::new(),
            thresholds: TunableThresholds::default(),
            recent_assessments: Vec::new(),
        }
    }

    /// Check if a consolidation is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Get the monitoring dashboard.
    pub fn dashboard(&self) -> &ConsolidationDashboard {
        &self.dashboard
    }

    /// Get the current tunable thresholds.
    pub fn thresholds(&self) -> &TunableThresholds {
        &self.thresholds
    }

    /// Run consolidation with existing semantic memories for dedup.
    pub fn consolidate_with_context(
        &mut self,
        candidates: &[BaseMemory],
        existing_semantics: &[(String, Vec<f32>)],
    ) -> CortexResult<ConsolidationResult> {
        // Acquire the single-execution guard.
        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(CortexError::ConsolidationError(
                ConsolidationError::MergeFailed {
                    reason: "consolidation already in progress".to_string(),
                },
            ));
        }

        let result = pipeline::run_pipeline(
            candidates,
            self.embedding_provider.as_ref(),
            existing_semantics,
        );

        // Release the guard.
        self.is_running.store(false, Ordering::SeqCst);

        let result = result?;

        // Assess quality and record.
        let assessment = monitoring::assess_quality(&result.metrics);
        self.dashboard
            .record_run(result.metrics.clone(), assessment.clone());
        self.recent_assessments.push(assessment);

        // Auto-tuning check.
        let adjustments =
            monitoring::auto_tuning::maybe_tune(&mut self.thresholds, &self.recent_assessments);
        for adj in &adjustments {
            info!(
                param = %adj.parameter,
                old = adj.old_value,
                new = adj.new_value,
                reason = %adj.reason,
                "auto-tuning adjustment"
            );
        }

        Ok(result)
    }
}

impl IConsolidator for ConsolidationEngine {
    fn consolidate(&self, candidates: &[BaseMemory]) -> CortexResult<ConsolidationResult> {
        // Acquire the single-execution guard.
        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(CortexError::ConsolidationError(
                ConsolidationError::MergeFailed {
                    reason: "consolidation already in progress".to_string(),
                },
            ));
        }

        let result = pipeline::run_pipeline(
            candidates,
            self.embedding_provider.as_ref(),
            &[], // No existing semantics in the basic trait interface.
        );

        // Release the guard.
        self.is_running.store(false, Ordering::SeqCst);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use cortex_core::memory::types::EpisodicContent;
    use cortex_core::memory::*;
    use cortex_core::traits::IEmbeddingProvider;

    /// Simple test embedding provider.
    struct TestEmbedder;

    impl IEmbeddingProvider for TestEmbedder {
        fn embed(&self, _text: &str) -> CortexResult<Vec<f32>> {
            Ok(vec![0.5; 64])
        }
        fn embed_batch(&self, texts: &[String]) -> CortexResult<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.5; 64]).collect())
        }
        fn dimensions(&self) -> usize {
            64
        }
        fn name(&self) -> &str {
            "test"
        }
        fn is_available(&self) -> bool {
            true
        }
    }

    fn make_old_episodic(summary: &str) -> BaseMemory {
        let content = TypedContent::Episodic(EpisodicContent {
            interaction: summary.to_string(),
            context: "ctx".to_string(),
            outcome: None,
        });
        let now = Utc::now();
        BaseMemory {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type: MemoryType::Episodic,
            content: content.clone(),
            summary: summary.to_string(),
            transaction_time: now - Duration::days(10),
            valid_time: now - Duration::days(10),
            valid_until: None,
            confidence: Confidence::new(0.8),
            importance: Importance::Normal,
            last_accessed: now,
            access_count: 3,
            linked_patterns: vec![],
            linked_constraints: vec![],
            linked_files: vec![],
            linked_functions: vec![],
            tags: vec!["rust".to_string()],
            archived: false,
            superseded_by: None,
            supersedes: None,
            content_hash: BaseMemory::compute_content_hash(&content).unwrap(),
        }
    }

    #[test]
    fn engine_consolidates_empty_candidates() {
        let engine = ConsolidationEngine::new(Box::new(TestEmbedder));
        let result = engine.consolidate(&[]).unwrap();
        assert!(result.created.is_empty());
        assert!(result.archived.is_empty());
    }

    #[test]
    fn engine_rejects_concurrent_runs() {
        let engine = ConsolidationEngine::new(Box::new(TestEmbedder));
        // Simulate a running consolidation.
        engine.is_running.store(true, Ordering::SeqCst);
        let result = engine.consolidate(&[]);
        assert!(result.is_err());
        engine.is_running.store(false, Ordering::SeqCst);
    }

    #[test]
    fn engine_processes_eligible_memories() {
        let engine = ConsolidationEngine::new(Box::new(TestEmbedder));
        let memories: Vec<BaseMemory> = (0..5)
            .map(|i| make_old_episodic(&format!("Rust memory safety topic {}", i)))
            .collect();
        let result = engine.consolidate(&memories).unwrap();
        // With identical embeddings, HDBSCAN should cluster them.
        // The exact result depends on HDBSCAN behavior with identical vectors.
        assert!(result.metrics.precision > 0.0);
    }
}
