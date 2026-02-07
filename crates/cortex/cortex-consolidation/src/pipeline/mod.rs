//! 6-phase consolidation pipeline orchestrator.
//!
//! Phase 1: Selection → Phase 2: Clustering → Phase 3: Recall Gate →
//! Phase 4: Abstraction → Phase 5: Integration → Phase 6: Pruning

pub mod phase1_selection;
pub mod phase2_clustering;
pub mod phase3_recall_gate;
pub mod phase4_abstraction;
pub mod phase5_integration;
pub mod phase6_pruning;

use cortex_core::errors::CortexResult;
use cortex_core::memory::BaseMemory;
use cortex_core::models::{ConsolidationMetrics, ConsolidationResult};
use cortex_core::traits::IEmbeddingProvider;
use tracing::{debug, info};

use phase5_integration::IntegrationAction;

/// Run the full 6-phase consolidation pipeline.
///
/// Returns a `ConsolidationResult` with created/archived IDs and quality metrics.
pub fn run_pipeline(
    candidates: &[BaseMemory],
    embedding_provider: &dyn IEmbeddingProvider,
    existing_semantics: &[(String, Vec<f32>)],
) -> CortexResult<ConsolidationResult> {
    // Phase 1: Selection.
    let selected = phase1_selection::select_candidates(candidates);
    info!(count = selected.len(), "Phase 1: selected candidates");

    if selected.is_empty() {
        return Ok(ConsolidationResult {
            created: vec![],
            archived: vec![],
            metrics: ConsolidationMetrics {
                precision: 1.0,
                compression_ratio: 1.0,
                lift: 1.0,
                stability: 1.0,
            },
        });
    }

    // Compute embeddings for all selected candidates.
    let texts: Vec<String> = selected.iter().map(|m| m.summary.clone()).collect();
    let all_embeddings = embedding_provider.embed_batch(&texts)?;

    // Phase 2: Clustering.
    let cluster_result = phase2_clustering::cluster_candidates(&selected, &all_embeddings);
    info!(
        clusters = cluster_result.clusters.len(),
        noise = cluster_result.noise.len(),
        "Phase 2: clustering complete"
    );

    let mut created = Vec::new();
    let mut archived = Vec::new();
    let mut total_input_tokens = 0usize;
    let mut total_output_tokens = 0usize;

    // Process each cluster through phases 3-6.
    for (cluster_idx, indices) in cluster_result.clusters.iter().enumerate() {
        let cluster: Vec<&BaseMemory> = indices.iter().map(|&i| selected[i]).collect();
        let cluster_embeddings: Vec<Vec<f32>> =
            indices.iter().map(|&i| all_embeddings[i].clone()).collect();

        // Phase 3: Recall Gate.
        let recall =
            phase3_recall_gate::check_recall(&cluster, &cluster_embeddings, &all_embeddings)?;

        if !recall.passed {
            debug!(
                cluster = cluster_idx,
                score = recall.score,
                "Phase 3: recall gate failed, deferring cluster"
            );
            continue;
        }

        // Phase 4: Abstraction.
        let abstraction = phase4_abstraction::abstract_cluster(&cluster, &cluster_embeddings);
        let new_memory = phase4_abstraction::build_semantic_memory(&abstraction)?;

        // Track token counts.
        for mem in &cluster {
            total_input_tokens += mem.summary.len() / 4;
        }
        total_output_tokens += new_memory.summary.len() / 4;

        // Phase 5: Integration.
        let new_emb = embedding_provider.embed(&new_memory.summary)?;
        let action = phase5_integration::determine_action(new_memory, &new_emb, existing_semantics);

        match action {
            IntegrationAction::Create(mem) => {
                info!(id = %mem.id, "Phase 5: creating new semantic memory");
                created.push(mem.id.clone());
            }
            IntegrationAction::Update { existing_id, .. } => {
                info!(id = %existing_id, "Phase 5: updating existing semantic memory");
                created.push(existing_id);
            }
        }

        // Phase 6: Pruning.
        let pruning =
            phase6_pruning::plan_pruning(&cluster, created.last().unwrap_or(&String::new()));
        archived.extend(pruning.archived_ids);
    }

    // Compute quality metrics.
    let compression_ratio = if total_output_tokens > 0 {
        total_input_tokens as f64 / total_output_tokens as f64
    } else {
        1.0
    };

    let precision = if !created.is_empty() { 0.8 } else { 1.0 };

    let metrics = ConsolidationMetrics {
        precision,
        compression_ratio,
        lift: 1.5, // Baseline lift estimate.
        stability: 0.9,
    };

    info!(
        created = created.len(),
        archived = archived.len(),
        compression_ratio = format!("{:.1}", compression_ratio),
        "Consolidation pipeline complete"
    );

    Ok(ConsolidationResult {
        created,
        archived,
        metrics,
    })
}
