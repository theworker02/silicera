//! Opt-in anonymous fleet-learning schema (no network by default).
//!
//! Participation must be explicit. A fleet recommendation is never treated as
//! optimal — only as a **candidate** for local measurement.

use serde::{Deserialize, Serialize};

use crate::hnep::{Confidence, HnepProfile};
use crate::Result;

/// Format id for anonymized experiment shares.
pub const FLEET_FORMAT: &str = "silicera-fleet-share";

/// Schema version.
pub const FLEET_VERSION: u32 = 1;

/// One anonymized workload outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetWorkloadShare {
    /// Workload name.
    pub workload: String,
    /// Winner variant id.
    pub winner: String,
    /// Confidence.
    pub confidence: String,
    /// Optional median ns (no percentages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner_median_ns: Option<f64>,
}

/// Sanitized shareable record (no hostnames, paths, or serials).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetShare {
    /// Format magic.
    pub format: String,
    /// Schema version.
    pub version: u32,
    /// Silicera version.
    pub silicera_version: String,
    /// Hardware class fingerprint (already non-secret by design).
    pub fingerprint: String,
    /// Microarchitecture tag when parseable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub microarchitecture: Option<String>,
    /// OS family only.
    pub os_family: String,
    /// Workload outcomes.
    pub workloads: Vec<FleetWorkloadShare>,
    /// Explicit opt-in acknowledgement embedded in the artifact.
    pub opt_in: bool,
    /// Caveats.
    pub caveats: Vec<String>,
}

impl FleetShare {
    /// Build a share from an HNEP. Requires `opt_in == true`.
    pub fn from_hnep(profile: &HnepProfile, opt_in: bool) -> Result<Self> {
        if !opt_in {
            return Err(crate::SiliceraError::Parse(
                "fleet share requires explicit opt-in (pass --opt-in)".into(),
            ));
        }
        profile.verify_integrity()?;
        let fp = profile.header.fingerprint.clone();
        let microarchitecture = fp.split(':').nth(2).map(|s| s.to_ascii_lowercase());
        let workloads = profile
            .workloads
            .iter()
            .filter(|w| w.confidence != Confidence::Inconclusive)
            .map(|w| FleetWorkloadShare {
                workload: w.name.clone(),
                winner: w.winner.clone(),
                confidence: w.confidence.label().into(),
                winner_median_ns: w.winner_median_ns,
            })
            .collect();
        Ok(Self {
            format: FLEET_FORMAT.into(),
            version: FLEET_VERSION,
            silicera_version: crate::VERSION.into(),
            fingerprint: fp,
            microarchitecture,
            os_family: profile.environment.os.clone(),
            workloads,
            opt_in: true,
            caveats: vec![
                "Opt-in only; Silicera does not upload this automatically.".into(),
                "A fleet recommendation is a CANDIDATE — the local machine must prove it.".into(),
                "No hostnames, user paths, or hardware serials are included.".into(),
            ],
        })
    }

    /// Pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Write to path.
    pub fn write_to(&self, path: &std::path::Path) -> Result<()> {
        std::fs::write(path, self.to_json_pretty()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::EnvironmentSnapshot;
    use crate::hnep::{HnepHeader, IntegrityDigest, WorkloadEntry};

    #[test]
    fn requires_opt_in() {
        let mut profile = HnepProfile {
            header: HnepHeader {
                format: crate::hnep::HNEP_FORMAT.into(),
                version: crate::hnep::HNEP_VERSION,
                silicera_version: crate::VERSION.into(),
                created_at: "1970-01-01T00:00:00Z".into(),
                fingerprint: "SLC:AMD:ZEN5:00:00:00:aaaaaaaaaaaaaaaa:bbbbbbbbbbbbbbbb".into(),
                label: "t".into(),
            },
            environment: EnvironmentSnapshot::capture(),
            workloads: vec![WorkloadEntry {
                name: "integer".into(),
                winner: "baseline".into(),
                confidence: Confidence::High,
                rationale: "test".into(),
                winner_median_ns: Some(100.0),
                baseline_median_ns: Some(100.0),
            }],
            size_classes: vec![],
            decision_tree: None,
            digest: IntegrityDigest {
                alg: "sha256".into(),
                hex: String::new(),
            },
        };
        profile.recompute_digest().unwrap();
        assert!(FleetShare::from_hnep(&profile, false).is_err());
        let share = FleetShare::from_hnep(&profile, true).unwrap();
        assert!(share.opt_in);
        assert_eq!(share.workloads.len(), 1);
    }
}
