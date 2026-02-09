//! Module boundary detection and import graph construction.

use drift_core::types::collections::{FxHashMap, FxHashSet};

use super::types::ImportGraph;

/// Builds an import graph from file-level import data.
///
/// Groups files into modules (by top-level directory) and constructs
/// directed edges representing inter-module dependencies.
pub struct ImportGraphBuilder {
    /// File → list of imported file paths.
    file_imports: FxHashMap<String, Vec<String>>,
    /// File → number of abstract types (interfaces, abstract classes, traits).
    file_abstract_counts: FxHashMap<String, u32>,
    /// File → total type count.
    file_type_counts: FxHashMap<String, u32>,
    /// Module depth: how many path segments define a module boundary.
    module_depth: usize,
}

impl ImportGraphBuilder {
    /// Create a new builder with the given module depth.
    /// `module_depth = 1` means top-level directories are modules.
    pub fn new(module_depth: usize) -> Self {
        Self {
            file_imports: FxHashMap::default(),
            file_abstract_counts: FxHashMap::default(),
            file_type_counts: FxHashMap::default(),
            module_depth: module_depth.max(1),
        }
    }

    /// Add a file and its imports.
    pub fn add_file(&mut self, file: &str, imports: &[String]) {
        self.file_imports.insert(file.to_string(), imports.to_vec());
    }

    /// Set abstract/total type counts for a file.
    pub fn set_type_counts(&mut self, file: &str, abstract_count: u32, total_count: u32) {
        self.file_abstract_counts.insert(file.to_string(), abstract_count);
        self.file_type_counts.insert(file.to_string(), total_count);
    }

    /// Build the import graph.
    pub fn build(&self) -> ImportGraph {
        let mut module_set: FxHashSet<String> = FxHashSet::default();
        let mut edges: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
        let mut abstract_counts: FxHashMap<String, u32> = FxHashMap::default();
        let mut total_type_counts: FxHashMap<String, u32> = FxHashMap::default();

        // Collect all modules
        for file in self.file_imports.keys() {
            let module = self.file_to_module(file);
            module_set.insert(module);
        }

        // Build edges and aggregate type counts
        for (file, imports) in &self.file_imports {
            let src_module = self.file_to_module(file);

            // Aggregate type counts
            if let Some(&ac) = self.file_abstract_counts.get(file) {
                *abstract_counts.entry(src_module.clone()).or_default() += ac;
            }
            if let Some(&tc) = self.file_type_counts.get(file) {
                *total_type_counts.entry(src_module.clone()).or_default() += tc;
            }

            for import in imports {
                let dst_module = self.file_to_module(import);
                if src_module != dst_module {
                    edges.entry(src_module.clone()).or_default().insert(dst_module.clone());
                    module_set.insert(dst_module);
                }
            }
        }

        let modules: Vec<String> = module_set.into_iter().collect();
        let edge_map: FxHashMap<String, Vec<String>> = edges
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();

        ImportGraph {
            edges: edge_map,
            modules,
            abstract_counts,
            total_type_counts,
        }
    }

    /// Extract module name from a file path based on module_depth.
    fn file_to_module(&self, file: &str) -> String {
        let normalized = file.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        if parts.len() <= self.module_depth {
            parts.join("/")
        } else {
            parts[..self.module_depth].join("/")
        }
    }
}

impl Default for ImportGraphBuilder {
    fn default() -> Self {
        Self::new(1)
    }
}
