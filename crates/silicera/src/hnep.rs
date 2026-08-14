//! HNEP — Hardware-Native Execution Profile.
//!
//! HNEP files (`.hnep`) store measured specialization decisions for a machine
//! fingerprint class. They are **not** AMD knowledge packs:
//!
//! | Artifact | Role |
//! |----------|------|
//! | Knowledge pack (`profiles/amd/*.toml`) | Public architecture facts for validation |
//! | HNEP (`.hnep`) | Measured winners + dispatch tree for one fingerprint |
//!
//! # Format contract
//!
//! See `docs/concepts/hnep.md` (overview) and `docs/research/hnep-spec.md`
//! (normative field/semver contract). Format id: [`HNEP_FORMAT`], schema
//! version: [`HNEP_VERSION`].
//!
//! Wire format: JSON envelope with SHA-256 integrity digest over the canonical
//! payload (everything except `digest` itself).

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, SiliceraError};
use crate::fingerprint::Fingerprint;
use crate::hardware::EnvironmentSnapshot;
use crate::specialize::DecisionTree;
use crate::tournament::TournamentResult;

/// Schema version for HNEP files.
///
/// Bumped to **2** when `size_classes` became a first-class measured array
/// (still JSON; additive for readers that ignore unknown fields is not assumed —
/// consumers must check `version`).
pub const HNEP_VERSION: u32 = 2;

/// Minimum schema version this library can load (inclusive).
pub const HNEP_VERSION_MIN_SUPPORTED: u32 = 1;

/// Maximum schema version this library can load (inclusive).
pub const HNEP_VERSION_MAX_SUPPORTED: u32 = 2;

/// Magic / format identifier (stable).
pub const HNEP_FORMAT: &str = "silicera-hnep";

/// Confidence in a specialization decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    /// Strong statistical evidence.
    High,
    /// Plausible improvement; use with awareness.
    Medium,
    /// Weak / noisy evidence.
    Low,
    /// No reliable winner; use baseline.
    Inconclusive,
}

impl Confidence {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            Confidence::High => "HIGH",
            Confidence::Medium => "MEDIUM",
            Confidence::Low => "LOW",
            Confidence::Inconclusive => "INCONCLUSIVE",
        }
    }

    /// Numeric rank for comparisons.
    pub fn rank(self) -> u8 {
        match self {
            Confidence::High => 3,
            Confidence::Medium => 2,
            Confidence::Low => 1,
            Confidence::Inconclusive => 0,
        }
    }
}

/// Integrity digest wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityDigest {
    /// Algorithm id (`sha256`).
    pub alg: String,
    /// Hex-encoded digest.
    pub hex: String,
}

impl IntegrityDigest {
    /// Compute SHA-256 over bytes.
    pub fn sha256(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let full = hasher.finalize();
        Self {
            alg: "sha256".into(),
            hex: hex::encode(full),
        }
    }

    /// Verify against bytes.
    pub fn verify(&self, bytes: &[u8]) -> Result<()> {
        let expected = Self::sha256(bytes);
        if expected.hex != self.hex || expected.alg != self.alg {
            return Err(SiliceraError::IntegrityFailed(format!(
                "expected {}, got {}",
                self.hex, expected.hex
            )));
        }
        Ok(())
    }
}

/// HNEP file header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnepHeader {
    /// Format magic — must equal [`HNEP_FORMAT`].
    pub format: String,
    /// Schema version — see [`HNEP_VERSION`].
    pub version: u32,
    /// Silicera library version that produced this profile.
    pub silicera_version: String,
    /// Creation timestamp (RFC3339).
    pub created_at: String,
    /// Target fingerprint string (`SLC:AMD:…`).
    pub fingerprint: String,
    /// Optional human label.
    pub label: String,
}

/// Per-workload specialization entry (fixed working set / domain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadEntry {
    /// Workload name.
    pub name: String,
    /// Selected variant id.
    pub winner: String,
    /// Confidence.
    pub confidence: Confidence,
    /// Rationale.
    pub rationale: String,
    /// Median ns of winner (informational; not a claim of speedup %).
    pub winner_median_ns: Option<f64>,
    /// Median ns of baseline when known.
    pub baseline_median_ns: Option<f64>,
}

/// Measured winner for a cache size-class (L1 / L2 / L3 / DRAM).
///
/// Thresholds are written from topology + measurement and compiled into the
/// decision tree. Different size classes may have different winners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeClassEntry {
    /// Class id: `L1`, `L2`, `L3`, or `DRAM` (stable uppercase).
    pub class: String,
    /// Upper threshold in bytes for this class (working set < threshold → class),
    /// except `DRAM` where threshold is the L3 boundary and working sets are ≥ L3.
    pub threshold_bytes: u64,
    /// Working-set size used during measurement (bytes).
    pub working_set_bytes: u64,
    /// Selected variant id for this class.
    pub winner: String,
    /// Confidence.
    pub confidence: Confidence,
    /// Rationale.
    pub rationale: String,
    /// Median ns of winner.
    pub winner_median_ns: Option<f64>,
    /// Median ns of baseline when known.
    pub baseline_median_ns: Option<f64>,
}

/// Full HNEP document (logical).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnepProfile {
    /// Header.
    pub header: HnepHeader,
    /// Environment at training time.
    pub environment: EnvironmentSnapshot,
    /// Workload entries.
    pub workloads: Vec<WorkloadEntry>,
    /// Size-class entries (v2+; empty on v1 loads after migration).
    #[serde(default)]
    pub size_classes: Vec<SizeClassEntry>,
    /// Optional decision tree for size-dependent dispatch (compiled from
    /// `size_classes` + topology thresholds when present).
    pub decision_tree: Option<DecisionTree>,
    /// Integrity digest over canonical payload (header+env+workloads+size_classes+tree).
    pub digest: IntegrityDigest,
}

/// Payload covered by the digest (excludes digest itself).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnepPayload {
    header: HnepHeader,
    environment: EnvironmentSnapshot,
    workloads: Vec<WorkloadEntry>,
    #[serde(default)]
    size_classes: Vec<SizeClassEntry>,
    decision_tree: Option<DecisionTree>,
}

impl HnepProfile {
    /// Build a new profile from tournament results and optional size-class entries.
    pub fn from_tournaments(
        fingerprint: &Fingerprint,
        environment: EnvironmentSnapshot,
        results: &[TournamentResult],
        size_classes: Vec<SizeClassEntry>,
        decision_tree: Option<DecisionTree>,
        label: impl Into<String>,
    ) -> Result<Self> {
        let workloads: Vec<WorkloadEntry> = results
            .iter()
            .map(|r| {
                let winner_rec = r.records.iter().find(|x| x.id == r.winner);
                let baseline_rec = r.records.iter().find(|x| x.id.0 == "baseline");
                WorkloadEntry {
                    name: r.name.clone(),
                    winner: r.winner.0.clone(),
                    confidence: r.confidence,
                    rationale: r.rationale.clone(),
                    winner_median_ns: winner_rec.map(|w| w.summary.median_ns),
                    baseline_median_ns: baseline_rec.map(|b| b.summary.median_ns),
                }
            })
            .collect();

        let header = HnepHeader {
            format: HNEP_FORMAT.into(),
            version: HNEP_VERSION,
            silicera_version: crate::VERSION.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            fingerprint: fingerprint.value.clone(),
            label: label.into(),
        };

        let payload = HnepPayload {
            header: header.clone(),
            environment: environment.clone(),
            workloads: workloads.clone(),
            size_classes: size_classes.clone(),
            decision_tree: decision_tree.clone(),
        };
        let canonical = serde_json::to_vec(&payload)?;
        let digest = IntegrityDigest::sha256(&canonical);

        Ok(Self {
            header,
            environment,
            workloads,
            size_classes,
            decision_tree,
            digest,
        })
    }

    /// Recompute digest after structural edits (tests / migrations).
    pub fn recompute_digest(&mut self) -> Result<()> {
        let payload = HnepPayload {
            header: self.header.clone(),
            environment: self.environment.clone(),
            workloads: self.workloads.clone(),
            size_classes: self.size_classes.clone(),
            decision_tree: self.decision_tree.clone(),
        };
        let canonical = serde_json::to_vec(&payload)?;
        self.digest = IntegrityDigest::sha256(&canonical);
        Ok(())
    }

    /// Verify integrity digest.
    pub fn verify_integrity(&self) -> Result<()> {
        let payload = HnepPayload {
            header: self.header.clone(),
            environment: self.environment.clone(),
            workloads: self.workloads.clone(),
            size_classes: self.size_classes.clone(),
            decision_tree: self.decision_tree.clone(),
        };
        let canonical = serde_json::to_vec(&payload)?;
        self.digest.verify(&canonical)
    }

    /// Write to path as JSON `.hnep`.
    pub fn write_to(&self, path: &Path) -> Result<()> {
        self.verify_integrity()?;
        let text = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Read and verify from path.
    pub fn read_from(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::parse_str(&text)
    }

    /// Parse from string and verify digest.
    pub fn parse_str(text: &str) -> Result<Self> {
        let mut profile: HnepProfile = serde_json::from_str(text)?;
        if profile.header.format != HNEP_FORMAT {
            return Err(SiliceraError::Parse(format!(
                "unexpected format {}",
                profile.header.format
            )));
        }
        if profile.header.version < HNEP_VERSION_MIN_SUPPORTED
            || profile.header.version > HNEP_VERSION_MAX_SUPPORTED
        {
            return Err(SiliceraError::Parse(format!(
                "unsupported HNEP version {} (supported {}..={})",
                profile.header.version, HNEP_VERSION_MIN_SUPPORTED, HNEP_VERSION_MAX_SUPPORTED
            )));
        }
        // v1 files lack size_classes in the digest payload historically — accept
        // empty default and verify against the payload shape we serialize now.
        // If an old file was hashed without size_classes key, re-hash fails;
        // migrate by re-digesting when version==1 and size_classes empty after
        // a soft verify attempt.
        if let Err(e) = profile.verify_integrity() {
            if profile.header.version == 1 && profile.size_classes.is_empty() {
                // Retry with legacy payload (no size_classes field in digest).
                #[derive(Serialize)]
                struct LegacyPayload {
                    header: HnepHeader,
                    environment: EnvironmentSnapshot,
                    workloads: Vec<WorkloadEntry>,
                    decision_tree: Option<DecisionTree>,
                }
                let legacy = LegacyPayload {
                    header: profile.header.clone(),
                    environment: profile.environment.clone(),
                    workloads: profile.workloads.clone(),
                    decision_tree: profile.decision_tree.clone(),
                };
                let canonical = serde_json::to_vec(&legacy)?;
                profile.digest.verify(&canonical).map_err(|_| e)?;
                // Upgrade in-memory to v2 shape with empty size_classes; digest
                // remains the legacy one until rewrite — mark version for clarity
                // but keep digest matching legacy bytes for verify_integrity via
                // dual path. Prefer rewriting digest to v2 on next write.
                profile.header.version = HNEP_VERSION;
                profile.recompute_digest()?;
            } else {
                return Err(e);
            }
        }
        Ok(profile)
    }

    /// Upsert a workload entry and recompute the integrity digest.
    pub fn upsert_workload(&mut self, entry: WorkloadEntry) -> Result<()> {
        if let Some(slot) = self.workloads.iter_mut().find(|w| w.name == entry.name) {
            *slot = entry;
        } else {
            self.workloads.push(entry);
        }
        self.recompute_digest()
    }

    /// Overall confidence = minimum across workloads and size-classes (conservative).
    pub fn overall_confidence(&self) -> Confidence {
        let w = self.workloads.iter().map(|w| w.confidence);
        let s = self.size_classes.iter().map(|c| c.confidence);
        w.chain(s)
            .min_by_key(|c| c.rank())
            .unwrap_or(Confidence::Inconclusive)
    }

    /// Check staleness against a fresh environment snapshot.
    ///
    /// Prefer [`crate::staleness::assess_staleness`] for full reports with
    /// retrain recommendations. This method remains a hard-error gate for
    /// callers that want `Result`.
    pub fn check_staleness(&self, now: &EnvironmentSnapshot, max_age_days: i64) -> Result<()> {
        let policy = crate::staleness::StalenessPolicy {
            soft_age_days: max_age_days,
            hard_age_days: max_age_days,
            watch_logical_cpus: true,
        };
        let report = crate::staleness::assess_staleness(self, now, None, &policy);
        if report.severity == crate::staleness::DriftSeverity::Hard {
            let detail = report
                .signals
                .iter()
                .map(|s| format!("{}: {}", s.field, s.note))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(SiliceraError::StaleProfile(detail));
        }
        Ok(())
    }

    /// Soft/hard staleness report (never invents winners).
    pub fn staleness_report(
        &self,
        now: &EnvironmentSnapshot,
        live_fingerprint: Option<&str>,
        policy: &crate::staleness::StalenessPolicy,
    ) -> crate::staleness::StalenessReport {
        crate::staleness::assess_staleness(self, now, live_fingerprint, policy)
    }

    /// Look up size-class winner by class id.
    pub fn size_class_winner(&self, class: &str) -> Option<&SizeClassEntry> {
        self.size_classes.iter().find(|c| c.class == class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::Fingerprint;
    use crate::knowledge::Microarch;
    use crate::topology::TopologyGraph;
    use crate::variant::VariantId;
    use crate::tournament::TournamentResult;
    use crate::measure::{MeasurementSummary, Stability};
    use crate::variant::VariantRecord;

    fn dummy_summary(median: f64) -> MeasurementSummary {
        MeasurementSummary {
            n: 10,
            min_ns: median,
            max_ns: median,
            mean_ns: median,
            median_ns: median,
            variance_ns2: 0.0,
            stddev_ns: 0.0,
            p25_ns: median,
            p75_ns: median,
            p95_ns: median,
            p99_ns: median,
            outlier_count: 0,
            stability: Stability::Stable,
            flags: vec![],
        }
    }

    #[test]
    fn hnep_roundtrip() {
        let fp = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &TopologyGraph::new());
        let results = vec![TournamentResult {
            name: "demo".into(),
            records: vec![VariantRecord {
                id: VariantId::new("baseline"),
                summary: dummy_summary(100.0),
                correct: true,
                regression: false,
                notes: vec![],
            }],
            winner: VariantId::new("baseline"),
            confidence: Confidence::Inconclusive,
            rationale: "test".into(),
        }];
        let profile = HnepProfile::from_tournaments(
            &fp,
            EnvironmentSnapshot::capture(),
            &results,
            Vec::new(),
            None,
            "test",
        )
        .unwrap();
        assert_eq!(profile.header.version, HNEP_VERSION);
        let json = serde_json::to_string(&profile).unwrap();
        let parsed = HnepProfile::parse_str(&json).unwrap();
        assert_eq!(parsed.header.fingerprint, fp.value);
        assert!(parsed.size_classes.is_empty());
    }

    #[test]
    fn size_class_roundtrip() {
        let fp = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &TopologyGraph::new());
        let sc = vec![SizeClassEntry {
            class: "L1".into(),
            threshold_bytes: 32 * 1024,
            working_set_bytes: 16 * 1024,
            winner: "scan".into(),
            confidence: Confidence::Medium,
            rationale: "measured".into(),
            winner_median_ns: Some(40.0),
            baseline_median_ns: Some(50.0),
        }];
        let profile = HnepProfile::from_tournaments(
            &fp,
            EnvironmentSnapshot::capture(),
            &[],
            sc,
            None,
            "sc",
        )
        .unwrap();
        let parsed = HnepProfile::parse_str(&serde_json::to_string(&profile).unwrap()).unwrap();
        assert_eq!(parsed.size_classes.len(), 1);
        assert_eq!(parsed.size_classes[0].winner, "scan");
    }
}

