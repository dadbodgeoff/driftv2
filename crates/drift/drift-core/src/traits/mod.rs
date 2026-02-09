//! Shared traits used across Drift crates.

pub mod cancellation;
pub mod decomposition;
pub mod weight_provider;

pub use cancellation::CancellationToken;
pub use decomposition::{DecompositionPriorProvider, NoOpPriorProvider};
pub use weight_provider::{
    AdaptiveWeightTable, MigrationPath, StaticWeightProvider, WeightProvider,
};
