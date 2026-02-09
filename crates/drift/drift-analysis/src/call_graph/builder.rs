//! CallGraphBuilder — parallel extraction via rayon, builds petgraph StableGraph.

use std::time::Instant;

use drift_core::errors::CallGraphError;
use drift_core::types::collections::FxHashMap;
use rayon::prelude::*;

use crate::parsers::types::{CallSite, FunctionInfo, ParseResult};

use super::resolution::resolve_call;
use super::types::{CallEdge, CallGraph, CallGraphStats, FunctionNode, Resolution};

/// Builder for constructing a call graph from parse results.
pub struct CallGraphBuilder {
    /// Maximum number of functions before switching to CTE fallback.
    pub in_memory_threshold: usize,
}

impl CallGraphBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            in_memory_threshold: 500_000,
        }
    }

    /// Create a builder with a custom in-memory threshold.
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            in_memory_threshold: threshold,
        }
    }

    /// Build a call graph from a set of parse results.
    ///
    /// Phase 1: Extract all functions into nodes (parallel via rayon).
    /// Phase 2: Resolve all call sites into edges (parallel per file).
    pub fn build(&self, parse_results: &[ParseResult]) -> Result<(CallGraph, CallGraphStats), CallGraphError> {
        let start = Instant::now();
        let mut graph = CallGraph::new();

        // Phase 1: Add all function nodes
        // Collect function nodes from all files in parallel
        let all_nodes: Vec<FunctionNode> = parse_results
            .par_iter()
            .flat_map_iter(|pr| {
                pr.functions.iter().map(move |f| FunctionNode {
                    file: pr.file.clone(),
                    name: f.name.clone(),
                    qualified_name: f.qualified_name.clone(),
                    language: pr.language.name().to_string(),
                    line: f.line,
                    end_line: f.end_line,
                    is_entry_point: false, // Detected later
                    is_exported: f.is_exported,
                    signature_hash: f.signature_hash,
                    body_hash: f.body_hash,
                })
            })
            .collect();

        for node in all_nodes {
            graph.add_function(node);
        }

        // Build lookup indices for resolution
        let mut name_index: FxHashMap<String, Vec<String>> = FxHashMap::default();
        let mut qualified_index: FxHashMap<String, String> = FxHashMap::default();
        let mut export_index: FxHashMap<String, Vec<String>> = FxHashMap::default();

        for pr in parse_results {
            for func in &pr.functions {
                let key = format!("{}::{}", pr.file, func.name);
                name_index.entry(func.name.clone()).or_default().push(key.clone());
                if let Some(ref qn) = func.qualified_name {
                    qualified_index.insert(qn.clone(), key.clone());
                }
                if func.is_exported {
                    export_index.entry(func.name.clone()).or_default().push(key);
                }
            }
        }

        // Phase 2: Resolve call sites into edges
        // Collect all (caller_key, call_site, file) tuples
        let call_entries: Vec<(String, &CallSite, &ParseResult)> = parse_results
            .iter()
            .flat_map(|pr| {
                pr.functions.iter().flat_map(move |func| {
                    let caller_key = format!("{}::{}", pr.file, func.name);
                    pr.call_sites
                        .iter()
                        .filter(move |cs| {
                            cs.line >= func.line && cs.line <= func.end_line
                        })
                        .map(move |cs| (caller_key.clone(), cs, pr))
                })
            })
            .collect();

        let mut resolution_counts: FxHashMap<String, usize> = FxHashMap::default();
        let mut resolved = 0usize;

        for (caller_key, call_site, pr) in &call_entries {
            if let Some(caller_idx) = graph.get_node(caller_key) {
                if let Some((callee_key, resolution)) = resolve_call(
                    call_site,
                    &pr.file,
                    &pr.imports,
                    &name_index,
                    &qualified_index,
                    &export_index,
                ) {
                    if let Some(callee_idx) = graph.get_node(&callee_key) {
                        let edge = CallEdge {
                            resolution,
                            confidence: resolution.default_confidence(),
                            call_site_line: call_site.line,
                        };
                        graph.add_edge(caller_idx, callee_idx, edge);
                        *resolution_counts.entry(resolution.name().to_string()).or_default() += 1;
                        resolved += 1;
                    }
                }
            }
        }

        // Detect entry points
        super::traversal::mark_entry_points(&mut graph, parse_results);

        let total_calls = call_entries.len();
        let stats = CallGraphStats {
            total_functions: graph.function_count(),
            total_edges: graph.edge_count(),
            entry_points: graph.graph.node_indices()
                .filter(|&idx| graph.graph[idx].is_entry_point)
                .count(),
            resolution_counts,
            resolution_rate: if total_calls > 0 {
                resolved as f64 / total_calls as f64
            } else {
                0.0
            },
            build_duration: start.elapsed(),
            cycles_detected: 0,
        };

        Ok((graph, stats))
    }
}

impl Default for CallGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
