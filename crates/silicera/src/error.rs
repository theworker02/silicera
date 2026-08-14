//! Error types for Silicera operations.

use thiserror::Error;

/// Convenient result alias.
pub type Result<T> = std::result::Result<T, SiliceraError>;

/// Errors produced by Silicera core operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SiliceraError {
    /// CPU vendor is not AMD, or CPUID is unavailable.
    #[error("unsupported CPU: {0}")]
    UnsupportedCpu(String),

    /// AMD CPU detected but microarchitecture is outside supported Zen3/Zen4/Zen5.
    #[error("unsupported AMD microarchitecture: {0}")]
    UnsupportedMicroarch(String),

    /// Topology or knowledge-pack validation failed.
    #[error("hardware validation failed: {0}")]
    ValidationFailed(String),

    /// Fingerprint / profile mismatch at load or dispatch time.
    #[error("profile mismatch: {0}")]
    ProfileMismatch(String),

    /// HNEP integrity digest does not match payload.
    #[error("integrity check failed: {0}")]
    IntegrityFailed(String),

    /// Measurement instability exceeded configured thresholds.
    #[error("measurement unstable: {0}")]
    MeasurementUnstable(String),

    /// Correctness validation rejected a variant.
    #[error("correctness failure: {0}")]
    CorrectnessFailure(String),

    /// Regression gate rejected a candidate (worse than baseline).
    #[error("regression rejected: {0}")]
    RegressionRejected(String),

    /// I/O or serialization failure.
    #[error("I/O error: {0}")]
    Io(String),

    /// Parse / format error.
    #[error("parse error: {0}")]
    Parse(String),

    /// Knowledge pack missing or incomplete.
    #[error("knowledge pack error: {0}")]
    KnowledgePack(String),

    /// Profile is stale relative to environment snapshot policy.
    #[error("stale profile: {0}")]
    StaleProfile(String),

    /// Generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for SiliceraError {
    fn from(e: std::io::Error) -> Self {
        SiliceraError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for SiliceraError {
    fn from(e: serde_json::Error) -> Self {
        SiliceraError::Parse(e.to_string())
    }
}

impl From<toml::de::Error> for SiliceraError {
    fn from(e: toml::de::Error) -> Self {
        SiliceraError::Parse(e.to_string())
    }
}
