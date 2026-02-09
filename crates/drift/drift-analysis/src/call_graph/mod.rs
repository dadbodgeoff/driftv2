//! Call Graph Builder — petgraph StableGraph, 6 resolution strategies, SQLite CTE fallback.
//!
//! Performance targets: Build <5s for 10K files, BFS <5ms, SQLite CTE <50ms.

pub mod types;
pub mod builder;
pub mod resolution;
pub mod traversal;
pub mod cte_fallback;
pub mod incremental;
pub mod di_support;

pub use types::{CallGraph, FunctionNode, CallEdge, Resolution, CallGraphStats};
pub use builder::CallGraphBuilder;
pub use traversal::{bfs_forward, bfs_inverse, detect_entry_points};
pub use incremental::IncrementalCallGraph;
