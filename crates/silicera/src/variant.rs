//! Variant trait and correctness validation.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SiliceraError};
use crate::measure::MeasurementSummary;

/// Stable identifier for a specialization variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantId(pub String);

impl VariantId {
    /// Construct from string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for VariantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for VariantId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Correctness check against a reference output.
#[derive(Debug, Clone)]
pub enum CorrectnessCheck<T> {
    /// Exact equality.
    Exact(T),
    /// Custom predicate (true = pass).
    Predicate(fn(&T) -> bool),
}

impl<T: PartialEq> CorrectnessCheck<T> {
    /// Validate observed output.
    pub fn validate(&self, observed: &T) -> Result<()> {
        match self {
            CorrectnessCheck::Exact(expected) => {
                if observed == expected {
                    Ok(())
                } else {
                    Err(SiliceraError::CorrectnessFailure(
                        "output does not match reference".into(),
                    ))
                }
            }
            CorrectnessCheck::Predicate(pred) => {
                if pred(observed) {
                    Ok(())
                } else {
                    Err(SiliceraError::CorrectnessFailure(
                        "predicate rejected output".into(),
                    ))
                }
            }
        }
    }
}

/// A candidate implementation specialized for a workload class.
pub trait Variant: Send + Sync {
    /// Output type produced by this variant (for correctness).
    type Output: Clone + Send;

    /// Stable id.
    fn id(&self) -> &VariantId;

    /// Short description for lab UI / explain.
    fn description(&self) -> &str;

    /// Execute once; return output for correctness.
    fn run(&self) -> Self::Output;
}

/// Named set of variants competing in a tournament.
#[derive(Debug, Clone)]
pub struct VariantSet {
    /// Workload / experiment name.
    pub name: String,
    /// Ordered variant ids (metadata only; runners hold the callables).
    pub variant_ids: Vec<VariantId>,
}

/// Metadata for a specialization target (used by `specialize_target!`).
#[derive(Debug, Clone, Copy)]
pub struct VariantSetMeta {
    /// Target name.
    pub name: &'static str,
    /// Baseline variant id.
    pub baseline: &'static str,
    /// Candidate variant ids.
    pub candidates: &'static [&'static str],
}

/// Per-variant measurement record after a tournament round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantRecord {
    /// Variant id.
    pub id: VariantId,
    /// Measurement summary.
    pub summary: MeasurementSummary,
    /// Correctness passed.
    pub correct: bool,
    /// Rejected for regression vs baseline.
    pub regression: bool,
    /// Notes.
    pub notes: Vec<String>,
}
