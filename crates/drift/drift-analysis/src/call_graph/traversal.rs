//! Forward/inverse BFS on petgraph, entry point detection.

use std::collections::VecDeque;

use drift_core::types::collections::{FxHashMap, FxHashSet};
use petgraph::graph::NodeIndex;
use petgraph::Direction;

use crate::parsers::types::ParseResult;

use super::types::{CallGraph, FunctionNode};

/// Forward BFS from a starting node — find all functions reachable from `start`.
pub fn bfs_forward(graph: &CallGraph, start: NodeIndex, max_depth: Option<usize>) -> Vec<NodeIndex> {
    bfs_directed(graph, start, Direction::Outgoing, max_depth)
}

/// Inverse BFS from a starting node — find all callers that can reach `start`.
pub fn bfs_inverse(graph: &CallGraph, start: NodeIndex, max_depth: Option<usize>) -> Vec<NodeIndex> {
    bfs_directed(graph, start, Direction::Incoming, max_depth)
}

/// Generic BFS in a given direction.
fn bfs_directed(
    graph: &CallGraph,
    start: NodeIndex,
    direction: Direction,
    max_depth: Option<usize>,
) -> Vec<NodeIndex> {
    let mut visited = FxHashSet::default();
    let mut queue = VecDeque::new();
    let mut result = Vec::new();

    visited.insert(start);
    queue.push_back((start, 0usize));

    while let Some((node, depth)) = queue.pop_front() {
        if node != start {
            result.push(node);
        }

        if let Some(max) = max_depth {
            if depth >= max {
                continue;
            }
        }

        for neighbor in graph.graph.neighbors_directed(node, direction) {
            if visited.insert(neighbor) {
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    result
}

/// Detect and mark entry points in the call graph.
///
/// 5 heuristic categories:
/// 1. Exported functions
/// 2. Main/index file functions
/// 3. Route handlers
/// 4. Test functions
/// 5. CLI entry points
pub fn detect_entry_points(graph: &CallGraph) -> Vec<NodeIndex> {
    let mut entry_points = Vec::new();

    for idx in graph.graph.node_indices() {
        let node = &graph.graph[idx];
        if is_entry_point(node) {
            entry_points.push(idx);
        }
    }

    entry_points
}

/// Mark entry points directly on the graph (mutable).
pub fn mark_entry_points(graph: &mut CallGraph, parse_results: &[ParseResult]) {
    // Build a set of route handler function names
    let mut route_handlers: FxHashSet<String> = FxHashSet::default();
    for pr in parse_results {
        for func in &pr.functions {
            for dec in &func.decorators {
                let dec_lower = dec.name.to_lowercase();
                if dec_lower.contains("route") || dec_lower.contains("get")
                    || dec_lower.contains("post") || dec_lower.contains("put")
                    || dec_lower.contains("delete") || dec_lower.contains("patch")
                    || dec_lower.contains("controller") || dec_lower.contains("api")
                    || dec_lower.contains("endpoint")
                {
                    route_handlers.insert(format!("{}::{}", pr.file, func.name));
                }
            }
        }
    }

    let indices: Vec<NodeIndex> = graph.graph.node_indices().collect();
    for idx in indices {
        let node = &graph.graph[idx];
        let key = format!("{}::{}", node.file, node.name);
        let is_entry = is_entry_point(node) || route_handlers.contains(&key);
        if is_entry {
            if let Some(node_mut) = graph.graph.node_weight_mut(idx) {
                node_mut.is_entry_point = true;
            }
        }
    }
}

/// Check if a function node is an entry point based on heuristics.
fn is_entry_point(node: &FunctionNode) -> bool {
    // 1. Exported functions
    if node.is_exported {
        return true;
    }

    // 2. Main/index file functions
    let file_lower = node.file.to_lowercase();
    if (file_lower.contains("main.") || file_lower.contains("index.")
        || file_lower.contains("app.") || file_lower.contains("server."))
        && matches!(node.name.as_str(), "main" | "run" | "start" | "init" | "bootstrap")
    {
        return true;
    }

    // 3. Test functions
    let name_lower = node.name.to_lowercase();
    if name_lower.starts_with("test_") || name_lower.starts_with("test")
        || name_lower.starts_with("it_") || name_lower.starts_with("spec_")
    {
        return true;
    }

    // 4. CLI entry points
    if matches!(node.name.as_str(), "main" | "cli" | "run_cli" | "parse_args") {
        return true;
    }

    false
}
