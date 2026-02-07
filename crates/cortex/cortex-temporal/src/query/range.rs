//! Temporal range query execution using Allen's interval algebra.

use cortex_core::errors::CortexResult;
use cortex_core::memory::BaseMemory;
use cortex_core::models::TemporalRangeQuery;
use cortex_storage::queries::temporal_ops;
use rusqlite::Connection;

use super::integrity::enforce_temporal_integrity;

/// Execute a temporal range query.
///
/// Uses Allen's interval algebra to find memories whose validity period
/// relates to the query range in the specified way:
/// - Overlaps: memory was valid at any point in [from, to]
/// - Contains: memory was valid for the entire [from, to]
/// - StartedDuring: memory became valid during [from, to]
/// - EndedDuring: memory stopped being valid during [from, to]
///
/// Optimized via temporal indexes on the memories table.
pub fn execute_range(
    conn: &Connection,
    query: &TemporalRangeQuery,
) -> CortexResult<Vec<BaseMemory>> {
    // Use the optimized SQL query with temporal indexes
    let memories = temporal_ops::get_memories_in_range(conn, query.from, query.to, query.mode)?;

    // Apply temporal integrity filter at the midpoint of the range
    // This ensures references are valid during the query period
    let midpoint = query.from + (query.to - query.from) / 2;
    let memories = enforce_temporal_integrity(memories, midpoint)?;

    Ok(memories)
}
