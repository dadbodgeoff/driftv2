use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The 13 relationship types between memories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    // Core (5)
    Supersedes,
    Supports,
    Contradicts,
    Related,
    DerivedFrom,
    // Semantic V2 (8)
    Owns,
    Affects,
    Blocks,
    Requires,
    References,
    LearnedFrom,
    AssignedTo,
    DependsOn,
}

impl RelationshipType {
    /// Total number of relationship types.
    pub const COUNT: usize = 13;

    /// All variants for iteration.
    pub const ALL: [RelationshipType; 13] = [
        Self::Supersedes,
        Self::Supports,
        Self::Contradicts,
        Self::Related,
        Self::DerivedFrom,
        Self::Owns,
        Self::Affects,
        Self::Blocks,
        Self::Requires,
        Self::References,
        Self::LearnedFrom,
        Self::AssignedTo,
        Self::DependsOn,
    ];
}

/// An edge in the relationship graph between two memories.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RelationshipEdge {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: RelationshipType,
    /// Strength of the relationship, 0.0–1.0.
    pub strength: f64,
    /// Evidence supporting this relationship.
    pub evidence: Vec<String>,
}
