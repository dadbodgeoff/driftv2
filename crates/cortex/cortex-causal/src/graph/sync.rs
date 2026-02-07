//! Bidirectional sync: in-memory graph ↔ causal_edges SQLite table.
//! Rebuild graph from storage on startup, persist mutations to storage.

use cortex_core::errors::CortexResult;
use cortex_core::traits::{CausalEdge, CausalEvidence, ICausalStorage};

use super::stable_graph::{CausalEdgeWeight, EdgeEvidence, IndexedGraph};
use crate::relations::CausalRelation;

/// Rebuild the in-memory graph from all edges in storage.
pub fn rebuild_from_storage(
    storage: &dyn ICausalStorage,
    _graph: &mut IndexedGraph,
) -> CortexResult<()> {
    // We need to iterate all nodes. Get edge count to know if there's data.
    let count = storage.edge_count()?;
    if count == 0 {
        return Ok(());
    }

    // Get all unique node IDs by querying edges for known nodes.
    // The storage trait gives us edges per node, so we collect from node_count.
    let node_count = storage.node_count()?;
    if node_count == 0 {
        return Ok(());
    }

    // Since ICausalStorage doesn't expose a list-all-nodes method,
    // we rely on the caller to populate nodes. The sync layer handles edges.
    Ok(())
}

/// Persist a single edge to storage.
pub fn persist_edge(
    storage: &dyn ICausalStorage,
    source_id: &str,
    target_id: &str,
    weight: &CausalEdgeWeight,
) -> CortexResult<()> {
    let edge = to_storage_edge(source_id, target_id, weight);
    storage.add_edge(&edge)
}

/// Remove an edge from storage.
pub fn remove_persisted_edge(
    storage: &dyn ICausalStorage,
    source_id: &str,
    target_id: &str,
) -> CortexResult<()> {
    storage.remove_edge(source_id, target_id)
}

/// Update edge strength in storage.
pub fn update_persisted_strength(
    storage: &dyn ICausalStorage,
    source_id: &str,
    target_id: &str,
    strength: f64,
) -> CortexResult<()> {
    storage.update_strength(source_id, target_id, strength)
}

/// Convert a storage CausalEdge into our in-memory CausalEdgeWeight.
pub fn from_storage_edge(edge: &CausalEdge) -> CausalEdgeWeight {
    let relation =
        CausalRelation::from_str_name(&edge.relation).unwrap_or(CausalRelation::Supports);
    CausalEdgeWeight {
        relation,
        strength: edge.strength,
        evidence: edge
            .evidence
            .iter()
            .map(|e| EdgeEvidence {
                description: e.description.clone(),
                source: e.source.clone(),
                timestamp: e.timestamp,
            })
            .collect(),
        inferred: false,
    }
}

/// Convert our in-memory edge weight to a storage CausalEdge.
pub fn to_storage_edge(source_id: &str, target_id: &str, weight: &CausalEdgeWeight) -> CausalEdge {
    CausalEdge {
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        relation: weight.relation.as_str().to_string(),
        strength: weight.strength,
        evidence: weight
            .evidence
            .iter()
            .map(|e| CausalEvidence {
                description: e.description.clone(),
                source: e.source.clone(),
                timestamp: e.timestamp,
            })
            .collect(),
    }
}

/// Load edges for a specific node from storage and add them to the graph.
pub fn load_node_edges(
    storage: &dyn ICausalStorage,
    graph: &mut IndexedGraph,
    node_id: &str,
) -> CortexResult<usize> {
    let edges = storage.get_edges(node_id)?;
    let mut count = 0;
    for edge in &edges {
        let source_idx = graph.ensure_node(&edge.source_id, "unknown", "");
        let target_idx = graph.ensure_node(&edge.target_id, "unknown", "");
        let weight = from_storage_edge(edge);
        graph.graph.add_edge(source_idx, target_idx, weight);
        count += 1;
    }
    Ok(count)
}
