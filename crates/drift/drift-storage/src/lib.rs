//! # drift-storage
//!
//! SQLite persistence layer for the Drift analysis engine.
//! WAL mode, write-serialized + read-pooled, batch writer,
//! keyset pagination, schema migrations.

#![allow(dead_code, unused)]

pub mod connection;
pub mod batch;
pub mod migrations;
pub mod queries;
pub mod pagination;
pub mod materialized;

pub use connection::DatabaseManager;
pub use batch::BatchWriter;
