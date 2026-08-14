//! Silicera Compiler Feedback (SCF) — experimental bridge from HNEP to tools.
//!
//! SCF is a **standalone JSON document** a future compiler / remark consumer can
//! read without linking the Silicera stack. It is derived from measured HNEP
//! data; it never invents winners.
//!
//! Format id: [`SCF_FORMAT`], version: [`SCF_VERSION`].
//! See `docs/research/compiler-feedback-format.md`.

use serde::{Deserialize, Serialize};

use crate::hnep::{Confidence, HnepProfile, SizeClassEntry, WorkloadEntry};
use crate::Result;

/// Wire format identifier (stable).
pub const SCF_FORMAT: &str = "silicera-compiler-feedback";

/// SCF schema version.
pub const SCF_VERSION: u32 = 1;

/// One workload recommendation for a compiler / multiversioning pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScfWorkload {
    /// Stable workload name from HNEP.
    pub name: String,
    /// Measured winner variant id (or baseline).
    pub winner: String,
    /// Confidence label.
    pub confidence: String,
    /// Optional baseline variant name when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    /// Optional median ns for winner (evidence, not marketing %).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner_median_ns: Option<f64>,
    /// Optional median ns for baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_median_ns: Option<f64>,
    /// Human rationale from tournament (may be empty).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

/// Size-class strategy hint (working-set thresholds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScfSizeClass {
    /// Class id (`L1`, `L2`, `L3`, `DRAM`).
    pub class: String,
    /// Upper threshold in bytes used for dispatch.
    pub threshold_bytes: u64,
    /// Working-set bytes used during measurement.
    pub working_set_bytes: u64,
    /// Winner variant.
    pub winner: String,
    /// Confidence.
    pub confidence: String,
}

/// Experimental compiler feedback document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerFeedback {
    /// Format magic.
    pub format: String,
    /// Schema version.
    pub version: u32,
    /// Silicera library version that produced this document.
    pub silicera_version: String,
    /// Machine fingerprint (identity of measurement environment — not auth).
    pub fingerprint: String,
    /// Host brand string when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_brand: Option<String>,
    /// Microarchitecture tag when known (`zen5`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub microarchitecture: Option<String>,
    /// Workload-level recommendations.
    pub workloads: Vec<ScfWorkload>,
    /// Size-class thresholds / winners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub size_classes: Vec<ScfSizeClass>,
    /// Decision-tree fallback variant when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_variant: Option<String>,
    /// Explicit caveats for consumers.
    pub caveats: Vec<String>,
}

impl CompilerFeedback {
    /// Build SCF from a validated HNEP profile.
    pub fn from_hnep(profile: &HnepProfile) -> Result<Self> {
        profile.verify_integrity()?;
        let fp = profile.header.fingerprint.clone();
        let microarchitecture = fp
            .split(':')
            .nth(2)
            .map(|s| s.to_ascii_lowercase());
        let workloads = profile
            .workloads
            .iter()
            .map(workload_to_scf)
            .collect::<Vec<_>>();
        let size_classes = profile
            .size_classes
            .iter()
            .map(size_class_to_scf)
            .collect::<Vec<_>>();
        let fallback_variant = profile
            .decision_tree
            .as_ref()
            .map(|t| t.fallback_variant.clone());
        Ok(Self {
            format: SCF_FORMAT.into(),
            version: SCF_VERSION,
            silicera_version: crate::VERSION.into(),
            fingerprint: fp,
            host_brand: None,
            microarchitecture,
            workloads,
            size_classes,
            fallback_variant,
            caveats: vec![
                "SCF is experimental; not an LLVM ABI.".into(),
                "Winners are machine-local measurements; re-verify on the target host.".into(),
                "Fingerprint identifies the specialization environment — not authentication."
                    .into(),
                "Community or cross-machine recommendations must be treated as candidates only."
                    .into(),
            ],
        })
    }

    /// Serialize to pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Write to a path.
    pub fn write_to(&self, path: &std::path::Path) -> Result<()> {
        std::fs::write(path, self.to_json_pretty()?)?;
        Ok(())
    }
}

fn workload_to_scf(w: &WorkloadEntry) -> ScfWorkload {
    ScfWorkload {
        name: w.name.clone(),
        winner: w.winner.clone(),
        confidence: conf_label(w.confidence),
        baseline: None,
        winner_median_ns: w.winner_median_ns,
        baseline_median_ns: w.baseline_median_ns,
        rationale: w.rationale.clone(),
    }
}

fn size_class_to_scf(c: &SizeClassEntry) -> ScfSizeClass {
    ScfSizeClass {
        class: c.class.clone(),
        threshold_bytes: c.threshold_bytes,
        working_set_bytes: c.working_set_bytes,
        winner: c.winner.clone(),
        confidence: conf_label(c.confidence),
    }
}

fn conf_label(c: Confidence) -> String {
    c.label().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::EnvironmentSnapshot;
    use crate::hnep::{HnepHeader, IntegrityDigest};

    #[test]
    fn scf_roundtrip_empty_workloads() {
        let mut profile = HnepProfile {
            header: HnepHeader {
                format: crate::hnep::HNEP_FORMAT.into(),
                version: crate::hnep::HNEP_VERSION,
                silicera_version: crate::VERSION.into(),
                created_at: "1970-01-01T00:00:00Z".into(),
                fingerprint: "SLC:AMD:ZEN5:00:00:00:deadbeefdeadbeef:cafebabecafebabe".into(),
                label: "test".into(),
            },
            environment: EnvironmentSnapshot::capture(),
            workloads: vec![],
            size_classes: vec![],
            decision_tree: None,
            digest: IntegrityDigest {
                alg: "sha256".into(),
                hex: String::new(),
            },
        };
        profile.recompute_digest().unwrap();
        let scf = CompilerFeedback::from_hnep(&profile).unwrap();
        assert_eq!(scf.format, SCF_FORMAT);
        assert_eq!(scf.version, SCF_VERSION);
        assert_eq!(scf.microarchitecture.as_deref(), Some("zen5"));
        let json = scf.to_json_pretty().unwrap();
        assert!(json.contains("silicera-compiler-feedback"));
    }
}
