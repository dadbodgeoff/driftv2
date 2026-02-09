//! Blast radius computation via transitive caller analysis.

use drift_core::types::collections::FxHashSet;
use petgraph::graph::NodeIndex;

use crate::call_graph::types::CallGraph;

use super::types::{BlastRadius, RiskScore};

/// Compute the blast radius for a function.
///
/// Uses inverse BFS to find all transitive callers — every function
/// that would be affected by a change to the target function.
pub fn compute_blast_radius(
    graph: &CallGraph,
    function_id: NodeIndex,
    max_callers_for_normalization: u32,
) -> BlastRadius {
    let (callers, max_depth) = transitive_callers(graph, function_id);
    let caller_count = callers.len() as u32;

    // Normalize blast radius to 0.0-1.0
    let blast_factor = (caller_count as f32 / max_callers_for_normalization as f32).min(1.0);

    let risk_score = RiskScore::compute(
        blast_factor,
        0.0, // Sensitivity: would need boundary data
        0.0, // Test coverage: would need test topology data
        0.0, // Complexity: would need AST analysis
        0.0, // Change frequency: would need git history
    );

    BlastRadius {
        function_id,
        transitive_callers: callers,
        caller_count,
        risk_score,
        max_depth,
    }
}

/// Compute blast radius for all functions in the graph.
pub fn compute_all_blast_radii(graph: &CallGraph) -> Vec<BlastRadius> {
    let max_callers = graph.function_count().max(1) as u32;

    graph
        .graph
        .node_indices()
        .map(|idx| compute_blast_radius(graph, idx, max_callers))
        .collect()
}

/// Find all transitive callers via inverse BFS.
/// Returns (callers, max_depth).
fn transitive_callers(graph: &CallGraph, start: NodeIndex) -> (Vec<NodeIndex>, u32) {
    let mut visited = FxHashSet::default();
    let mut queue = std::collections::VecDeque::new();
    let mut result = Vec::new();
    let mut max_depth = 0u32;

    visited.insert(start);
    queue.push_back((start, 0u32));

    while let Some((node, depth)) = queue.pop_front() {
        if node != start {
            result.push(node);
            if depth > max_depth {
                max_depth = depth;
            }
        }

        for caller in graph.graph.neighbors_directed(node, petgraph::Direction::Incoming) {
            if visited.insert(caller) {
                queue.push_back((caller, depth + 1));
            }
        }
    }

    (result, max_depth)
}
