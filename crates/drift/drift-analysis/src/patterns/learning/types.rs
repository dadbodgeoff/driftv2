//! Core types for the learning system.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::patterns::confidence::types::ConfidenceScore;

/// A discovered convention.
#[derive(Debug, Clone)]
pub struct Convention {
    /// Unique convention ID.
    pub id: String,
    /// Pattern ID this convention is based on.
    pub pattern_id: String,
    /// Convention category.
    pub category: ConventionCategory,
    /// Scope of the convention.
    pub scope: ConventionScope,
    /// Confidence score from Bayesian scoring.
    pub confidence_score: ConfidenceScore,
    /// Dominance ratio: this pattern's frequency / total alternatives.
    pub dominance_ratio: f64,
    /// Unix timestamp of discovery.
    pub discovery_date: u64,
    /// Unix timestamp of last observation.
    pub last_seen: u64,
    /// Current promotion status.
    pub promotion_status: PromotionStatus,
}

/// Convention categories based on spread and consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConventionCategory {
    /// High spread (≥80% of files), high confidence (Established tier).
    Universal,
    /// Moderate spread, project-scoped.
    ProjectSpecific,
    /// Rising momentum, growing adoption.
    Emerging,
    /// Falling momentum, declining usage.
    Legacy,
    /// Two patterns within 15% frequency of each other.
    Contested,
}

impl ConventionCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::ProjectSpecific => "project_specific",
            Self::Emerging => "emerging",
            Self::Legacy => "legacy",
            Self::Contested => "contested",
        }
    }
}

impl fmt::Display for ConventionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Scope of a convention.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConventionScope {
    /// Applies to the entire project.
    Project,
    /// Applies to a specific directory.
    Directory(String),
    /// Applies to a specific package/module.
    Package(String),
}

impl ConventionScope {
    pub fn name(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Directory(d) => d,
            Self::Package(p) => p,
        }
    }
}

impl fmt::Display for ConventionScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => write!(f, "project"),
            Self::Directory(d) => write!(f, "directory:{}", d),
            Self::Package(p) => write!(f, "package:{}", p),
        }
    }
}

/// Promotion status lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PromotionStatus {
    /// Newly discovered, not yet promoted.
    Discovered,
    /// Promoted to enforced convention.
    Approved,
    /// Explicitly rejected by user.
    Rejected,
    /// Expired due to inactivity.
    Expired,
}

impl PromotionStatus {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }
}

impl fmt::Display for PromotionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Configuration for the learning system.
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Minimum occurrences for a pattern to be considered a convention.
    pub min_occurrences: u64,
    /// Minimum dominance ratio (pattern frequency / total alternatives).
    pub dominance_threshold: f64,
    /// Minimum files for a pattern to be considered.
    pub min_files: u64,
    /// Spread threshold for Universal classification (≥80%).
    pub universal_spread_threshold: f64,
    /// Contested threshold: two patterns within this % of each other.
    pub contested_threshold: f64,
    /// Days before a convention expires if not seen.
    pub expiry_days: u64,
    /// File change threshold for triggering re-learning (>10%).
    pub relearn_threshold: f64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            dominance_threshold: 0.60,
            min_files: 2,
            universal_spread_threshold: 0.80,
            contested_threshold: 0.15,
            expiry_days: 90,
            relearn_threshold: 0.10,
        }
    }
}
