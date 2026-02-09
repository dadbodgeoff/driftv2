//! # drift-analysis
//!
//! Analysis engine for the Drift codebase analysis tool.
//! Contains scanner, parsers, engine, detectors, call graph,
//! boundaries, and language provider systems.

#![allow(dead_code, unused)]
#![allow(clippy::module_inception)]

pub mod scanner;
pub mod parsers;
pub mod engine;
pub mod detectors;
pub mod call_graph;
pub mod boundaries;
pub mod language_provider;
pub mod patterns;
pub mod graph;
pub mod structural;
pub mod enforcement;
pub mod advanced;
