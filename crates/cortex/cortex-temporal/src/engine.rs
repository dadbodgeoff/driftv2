//! TemporalEngine — central orchestrator implementing ITemporalEngine.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use cortex_core::config::TemporalConfig;
use cortex_core::errors::CortexResult;
use cortex_core::memory::BaseMemory;
use cortex_core::models::{
    AsOfQuery, DecisionReplay, DecisionReplayQuery, MemoryEvent, TemporalCausalQuery, TemporalDiff,
    TemporalDiffQuery, TemporalRangeQuery,
};
use cortex_core::traits::{ITemporalEngine, TemporalTraversalNode, TemporalTraversalResult};
use cortex_storage::pool::{ReadPool, WriteConnection};

use crate::event_store;
use crate::query;
use crate::snapshot;

/// The temporal reasoning engine.
///
/// Holds references to WriteConnection (event appends, snapshot creation)
/// and ReadPool (all temporal queries) per CR5.
pub struct TemporalEngine {
    pub(crate) writer: Arc<WriteConnection>,
    pub(crate) readers: Arc<ReadPool>,
    #[allow(dead_code)]
    pub(crate) config: TemporalConfig,
}

impl TemporalEngine {
    /// Create a new TemporalEngine.
    pub fn new(
        writer: Arc<WriteConnection>,
        readers: Arc<ReadPool>,
        config: TemporalConfig,
    ) -> Self {
        Self {
            writer,
            readers,
            config,
        }
    }
}

impl ITemporalEngine for TemporalEngine {
    async fn record_event(&self, event: MemoryEvent) -> CortexResult<u64> {
        let writer = self.writer.clone();
        event_store::append::append(&writer, &event).await
    }

    async fn get_events(
        &self,
        memory_id: &str,
        before: Option<DateTime<Utc>>,
    ) -> CortexResult<Vec<MemoryEvent>> {
        let readers = self.readers.clone();
        let mid = memory_id.to_string();
        event_store::query::get_events(&readers, &mid, before)
    }

    async fn reconstruct_at(
        &self,
        memory_id: &str,
        as_of: DateTime<Utc>,
    ) -> CortexResult<Option<BaseMemory>> {
        let readers = self.readers.clone();
        let mid = memory_id.to_string();
        snapshot::reconstruct::reconstruct_at(&readers, &mid, as_of)
    }

    async fn reconstruct_all_at(&self, as_of: DateTime<Utc>) -> CortexResult<Vec<BaseMemory>> {
        let readers = self.readers.clone();
        snapshot::reconstruct::reconstruct_all_at(&readers, as_of)
    }

    // Phase B: Temporal queries
    async fn query_as_of(&self, query: &AsOfQuery) -> CortexResult<Vec<BaseMemory>> {
        let readers = self.readers.clone();
        readers.with_conn(|conn| query::as_of::execute_as_of(conn, query))
    }

    async fn query_range(&self, query: &TemporalRangeQuery) -> CortexResult<Vec<BaseMemory>> {
        let readers = self.readers.clone();
        readers.with_conn(|conn| query::range::execute_range(conn, query))
    }

    async fn query_diff(&self, query: &TemporalDiffQuery) -> CortexResult<TemporalDiff> {
        let readers = self.readers.clone();
        readers.with_conn(|conn| query::diff::execute_diff(conn, query))
    }

    // Phase C: Decision replay + temporal causal
    async fn replay_decision(&self, query: &DecisionReplayQuery) -> CortexResult<DecisionReplay> {
        let readers = self.readers.clone();
        query::replay::execute_replay(&readers, query)
    }

    async fn query_temporal_causal(
        &self,
        query: &TemporalCausalQuery,
    ) -> CortexResult<TemporalTraversalResult> {
        let readers = self.readers.clone();
        let causal_result = query::temporal_causal::execute_temporal_causal(&readers, query)?;

        // Convert cortex_causal::TraversalResult → cortex_core::TemporalTraversalResult
        Ok(TemporalTraversalResult {
            origin_id: causal_result.origin_id,
            nodes: causal_result
                .nodes
                .into_iter()
                .map(|n| TemporalTraversalNode {
                    memory_id: n.memory_id,
                    depth: n.depth,
                    path_strength: n.path_strength,
                })
                .collect(),
            max_depth_reached: causal_result.max_depth_reached,
        })
    }
}
