//! Experiment reproducibility pack — export enough metadata to re-run later.
//!
//! Does not include personal paths or hardware serials.

use serde::{Deserialize, Serialize};
use crate::brand::{NAME, PHASE, VERSION};
use crate::hardware::{EnvironmentSnapshot, HardwareInfo};
use crate::Result;

/// Format id.
pub const REPRO_FORMAT: &str = "silicera-repro-pack";

/// Schema version.
pub const REPRO_VERSION: u32 = 1;

/// Compact reproducibility bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproPack {
    /// Format magic.
    pub format: String,
    /// Schema version.
    pub version: u32,
    /// Product name.
    pub product: String,
    /// Silicera version.
    pub silicera_version: String,
    /// Phase marker.
    pub phase: String,
    /// Experiment / command label.
    pub experiment: String,
    /// RFC3339 timestamp.
    pub created_at: String,
    /// Fingerprint when supported.
    pub fingerprint: Option<String>,
    /// Host brand.
    pub host_brand: String,
    /// Microarch tag when known.
    pub microarchitecture: Option<String>,
    /// Environment snapshot (no hostname).
    pub environment: EnvironmentSnapshot,
    /// Command-line / knobs used.
    pub parameters: serde_json::Value,
    /// Optional random seed.
    pub random_seed: Option<u64>,
    /// Optional result digest (hex) for an accompanying artifact.
    pub result_digest: Option<String>,
    /// Caveats.
    pub caveats: Vec<String>,
}

impl ReproPack {
    /// Build from live hardware + experiment metadata.
    pub fn capture(
        info: &HardwareInfo,
        experiment: impl Into<String>,
        parameters: serde_json::Value,
        random_seed: Option<u64>,
        result_digest: Option<String>,
    ) -> Self {
        let microarchitecture = info
            .fingerprint
            .as_ref()
            .and_then(|f| f.value.split(':').nth(2).map(|s| s.to_ascii_lowercase()));
        Self {
            format: REPRO_FORMAT.into(),
            version: REPRO_VERSION,
            product: NAME.into(),
            silicera_version: VERSION.into(),
            phase: PHASE.into(),
            experiment: experiment.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
            host_brand: info.brand.clone(),
            microarchitecture,
            environment: info.environment.clone(),
            parameters,
            random_seed,
            result_digest,
            caveats: vec![
                "Repro packs identify measurement conditions; they are not authentication.".into(),
                "Re-run on the same fingerprint class; do not assume cross-machine transfer.".into(),
                "No user home paths or hardware serial numbers are included.".into(),
            ],
        }
    }

    /// Pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Write to path.
    pub fn write_to(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json_pretty()?)?;
        Ok(())
    }
}
