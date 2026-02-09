//! Incremental call graph updates — re-extract only changed files.

use crate::parsers::types::ParseResult;

use super::builder::CallGraphBuilder;
use super::types::{CallGraph, CallGraphStats};

/// Incremental call graph manager.
///
/// Maintains a call graph and updates it incrementally when files change.
pub struct IncrementalCallGraph {
    graph: CallGraph,
    builder: CallGraphBuilder,
}

impl IncrementalCallGraph {
    /// Create a new incremental call graph.
    pub fn new() -> Self {
        Self {
            graph: CallGraph::new(),
            builder: CallGraphBuilder::new(),
        }
    }

    /// Get a reference to the current call graph.
    pub fn graph(&self) -> &CallGraph {
        &self.graph
    }

    /// Full build from scratch.
    pub fn full_build(
        &mut self,
        parse_results: &[ParseResult],
    ) -> Result<CallGraphStats, drift_core::errors::CallGraphError> {
        let (graph, stats) = self.builder.build(parse_results)?;
        self.graph = graph;
        Ok(stats)
    }

    /// Incremental update: remove edges for deleted/modified files, re-add for new/modified.
    ///
    /// - `added`: newly added files (parse results)
    /// - `modified`: modified files (parse results)
    /// - `removed`: paths of removed files
    /// - `all_results`: all current parse results (for re-resolution)
    pub fn update(
        &mut self,
        added: &[ParseResult],
        modified: &[ParseResult],
        removed: &[String],
        all_results: &[ParseResult],
    ) -> Result<CallGraphStats, drift_core::errors::CallGraphError> {
        // Remove nodes/edges for deleted files
        for path in removed {
            self.graph.remove_file(path);
        }

        // Remove nodes/edges for modified files (will be re-added)
        for pr in modified {
            self.graph.remove_file(&pr.file);
        }

        // Rebuild from all results (simpler and correct for now)
        // A more sophisticated approach would only re-resolve affected edges
        let (graph, stats) = self.builder.build(all_results)?;
        self.graph = graph;
        Ok(stats)
    }
}

impl Default for IncrementalCallGraph {
    fn default() -> Self {
        Self::new()
    }
}
