//! # drift-napi
//!
//! NAPI-RS v3 bindings for the Drift analysis engine.
//! Provides the TypeScript/JavaScript bridge layer.
//!
//! Architecture:
//! - `runtime` — `DriftRuntime` singleton via `OnceLock` (lock-free after init)
//! - `conversions` — Rust ↔ JS type conversions, error code mapping
//! - `bindings` — NAPI-exported functions (lifecycle, scanner)

#![allow(dead_code, unused)]

pub mod runtime;
pub mod conversions;
pub mod bindings;
