//! # silicera-runtime
//!
//! Lightweight HNEP load and cheap decision-tree dispatch.
//!
//! This crate intentionally does **not** depend on `silicera-lab`. Specialized
//! binaries embed or load a profile and select variants without re-running
//! tournaments.
//!
//! On fingerprint mismatch the runtime reports `PROFILE MISMATCH` and falls
//! back to the baseline variant unless `--strict-machine` / strict API is used.

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::path::Path;

use serde::{Deserialize, Serialize};
use silicera::brand::{BrandInfo, NAME};
use silicera::fingerprint::Fingerprint;
use silicera::hardware::{detect_hardware, HardwareInfo};
use silicera::hnep::HnepProfile;
use silicera::specialize::DecisionTree;
use silicera::{Result, SiliceraError};

#[cfg(feature = "c-abi")]
pub mod ffi;

/// Embedder-facing about / identity block (version, phase, brand, funding).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    /// Product name.
    pub name: String,
    /// Runtime crate version (same workspace semver).
    pub version: String,
    /// Phase marker.
    pub phase: String,
    /// Brand line.
    pub brand_line: String,
    /// SPDX license.
    pub license: String,
    /// Homepage.
    pub homepage: String,
    /// Repository URL.
    pub repository: String,
    /// Funding / thanks.dev URL.
    pub funding_url: String,
    /// Affiliation disclaimer.
    pub affiliation: String,
}

impl RuntimeInfo {
    /// Capture current runtime identity from the shared brand module.
    pub fn current() -> Self {
        let b = BrandInfo::current();
        Self {
            name: b.name.into(),
            version: b.version.into(),
            phase: b.phase.into(),
            brand_line: b.brand_line.into(),
            license: b.license.into(),
            homepage: b.homepage.into(),
            repository: b.repository.into(),
            funding_url: b.funding_url.into(),
            affiliation: b.affiliation.into(),
        }
    }

    /// Pretty JSON for tooling.
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Convenience: brand/version/phase for embedders.
pub fn runtime_about() -> RuntimeInfo {
    RuntimeInfo::current()
}

/// Runtime policy when the loaded profile does not match the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MismatchPolicy {
    /// Print/return mismatch and use baseline fallback.
    FallbackBaseline,
    /// Hard error — refuse to dispatch specialized variants.
    StrictMachine,
}

impl Default for MismatchPolicy {
    fn default() -> Self {
        Self::FallbackBaseline
    }
}

/// Outcome of loading a profile against the current machine.
#[derive(Debug, Clone)]
pub struct LoadedProfile {
    /// Parsed HNEP.
    pub profile: HnepProfile,
    /// Host hardware at load time.
    pub host: HardwareInfo,
    /// Whether fingerprint matched exactly.
    pub exact_match: bool,
    /// Whether silicon class matched (vendor/microarch/family/model).
    pub class_match: bool,
    /// Active mismatch policy.
    pub policy: MismatchPolicy,
    /// True if dispatch should use baseline only.
    pub force_baseline: bool,
    /// Operator-facing status line.
    pub status: String,
}

impl LoadedProfile {
    /// Load from path and compare to live hardware.
    pub fn load(path: &Path, policy: MismatchPolicy) -> Result<Self> {
        let profile = HnepProfile::read_from(path)?;
        let host = detect_hardware()?;
        Self::from_parts(profile, host, policy)
    }

    /// Construct from already-loaded pieces (tests / inject mock host).
    pub fn from_parts(
        profile: HnepProfile,
        host: HardwareInfo,
        policy: MismatchPolicy,
    ) -> Result<Self> {
        // Callers can construct profiles directly; do not trust their digest.
        let profile = HnepProfile::parse_str(&serde_json::to_string(&profile)?)?;
        let host_fp = match &host.fingerprint {
            Some(fp) => fp.clone(),
            None => {
                if policy == MismatchPolicy::StrictMachine {
                    return Err(SiliceraError::ProfileMismatch(format!(
                        "strict-machine ({NAME}): host unsupported ({})",
                        host.support.message()
                    )));
                }
                return Ok(Self {
                    status: format!(
                        "PROFILE MISMATCH ({NAME}): host unsupported ({})",
                        host.support.message()
                    ),
                    profile,
                    host,
                    exact_match: false,
                    class_match: false,
                    policy,
                    force_baseline: true,
                });
            }
        };

        let profile_fp = Fingerprint::parse(&profile.header.fingerprint)?;
        let exact_match = host_fp.exact_match(&profile_fp);
        let class_match = host_fp.same_silicon_class(&profile_fp);

        let (force_baseline, status) = if exact_match {
            (false, format!("profile match: exact fingerprint ({NAME})"))
        } else if class_match {
            match policy {
                MismatchPolicy::FallbackBaseline => (
                    true,
                    format!(
                        "PROFILE MISMATCH ({NAME}): topology/cache hash differs (class OK); falling back to baseline\n  profile: {}\n  host:    {}",
                        profile_fp, host_fp
                    ),
                ),
                MismatchPolicy::StrictMachine => {
                    return Err(SiliceraError::ProfileMismatch(format!(
                        "strict-machine ({NAME}): fingerprint mismatch\n  profile: {profile_fp}\n  host:    {host_fp}"
                    )));
                }
            }
        } else {
            match policy {
                MismatchPolicy::FallbackBaseline => (
                    true,
                    format!(
                        "PROFILE MISMATCH ({NAME}): silicon class differs; falling back to baseline\n  profile: {}\n  host:    {}",
                        profile_fp, host_fp
                    ),
                ),
                MismatchPolicy::StrictMachine => {
                    return Err(SiliceraError::ProfileMismatch(format!(
                        "strict-machine ({NAME}): silicon class mismatch\n  profile: {profile_fp}\n  host:    {host_fp}"
                    )));
                }
            }
        };

        Ok(Self {
            profile,
            host,
            exact_match,
            class_match,
            policy,
            force_baseline,
            status,
        })
    }

    /// Select variant for a workload by name (or baseline if forced).
    pub fn select_workload(&self, workload: &str) -> &str {
        if self.force_baseline {
            return "baseline";
        }
        self.profile
            .workloads
            .iter()
            .find(|w| w.name == workload)
            .map(|w| w.winner.as_str())
            .unwrap_or("baseline")
    }

    /// Evaluate size-dependent tree, or baseline on mismatch / missing tree.
    pub fn select_size(&self, size_bytes: u64) -> &str {
        if self.force_baseline {
            return "baseline";
        }
        match &self.profile.decision_tree {
            Some(tree) => tree.evaluate(size_bytes),
            None => "baseline",
        }
    }

    /// Access decision tree when present.
    pub fn decision_tree(&self) -> Option<&DecisionTree> {
        self.profile.decision_tree.as_ref()
    }

    /// Select a workload only when its winner is supported by the application
    /// and meets its confidence floor. Always provide a real baseline handler.
    /// The available IDs must already be filtered for the host's ISA support.
    pub fn guarded_workload<'a>(
        &'a self,
        workload: &str,
        available: &[&str],
        minimum: silicera::hnep::Confidence,
    ) -> DispatchDecision<'a> {
        if self.force_baseline {
            return DispatchDecision::baseline(DispatchReason::MachineMismatch);
        }
        let Some(entry) = self.profile.workloads.iter().find(|w| w.name == workload) else {
            return DispatchDecision::baseline(DispatchReason::MissingWorkload);
        };
        if entry.winner == "baseline" {
            return DispatchDecision::baseline(DispatchReason::ProfileBaseline);
        }
        if entry.confidence.rank() < minimum.rank() {
            return DispatchDecision::baseline(DispatchReason::InsufficientConfidence);
        }
        if !available.contains(&entry.winner.as_str()) {
            return DispatchDecision::baseline(DispatchReason::UnavailableVariant);
        }
        DispatchDecision {
            variant: &entry.winner,
            reason: DispatchReason::Selected,
        }
    }

    /// Guard size dispatch against variant IDs absent from the host-compatible
    /// application registry. This does not infer confidence from tree leaves.
    pub fn guarded_size<'a>(&'a self, bytes: u64, available: &[&str]) -> DispatchDecision<'a> {
        if self.force_baseline {
            return DispatchDecision::baseline(DispatchReason::MachineMismatch);
        }
        let Some(tree) = &self.profile.decision_tree else {
            return DispatchDecision::baseline(DispatchReason::MissingTree);
        };
        let variant = tree.evaluate(bytes);
        if variant == "baseline" {
            return DispatchDecision::baseline(DispatchReason::ProfileBaseline);
        }
        if !available.contains(&variant) {
            return DispatchDecision::baseline(DispatchReason::UnavailableVariant);
        }
        DispatchDecision {
            variant,
            reason: DispatchReason::Selected,
        }
    }
}

/// Stable, machine-readable explanation of a guarded dispatch decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchReason {
    /// A supported specialized implementation was selected.
    Selected,
    /// The profile explicitly selected baseline.
    ProfileBaseline,
    /// The machine does not match the profile.
    MachineMismatch,
    /// The workload is absent from the profile.
    MissingWorkload,
    /// No size decision tree was recorded.
    MissingTree,
    /// The workload confidence is below the requested floor.
    InsufficientConfidence,
    /// The application cannot execute this variant on this host.
    UnavailableVariant,
}

/// Allocation-free selection result for applications with a variant registry.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct DispatchDecision<'a> {
    /// Variant to execute; `baseline` must always be implemented by the caller.
    pub variant: &'a str,
    /// Why this variant was selected.
    pub reason: DispatchReason,
}

impl DispatchDecision<'_> {
    fn baseline(reason: DispatchReason) -> Self {
        Self {
            variant: "baseline",
            reason,
        }
    }
}

/// Convenience: dispatch API used by specialized binaries.
pub struct Dispatcher {
    loaded: LoadedProfile,
}

impl Dispatcher {
    /// Load profile with policy.
    pub fn open(path: &Path, policy: MismatchPolicy) -> Result<Self> {
        Ok(Self {
            loaded: LoadedProfile::load(path, policy)?,
        })
    }

    /// Status line (match / mismatch).
    pub fn status(&self) -> &str {
        &self.loaded.status
    }

    /// Whether baseline is forced.
    pub fn using_baseline(&self) -> bool {
        self.loaded.force_baseline
    }

    /// Select by workload name.
    pub fn workload(&self, name: &str) -> &str {
        self.loaded.select_workload(name)
    }

    /// Select by working-set size.
    pub fn size(&self, bytes: u64) -> &str {
        self.loaded.select_size(bytes)
    }

    /// Borrow loaded profile.
    pub fn loaded(&self) -> &LoadedProfile {
        &self.loaded
    }

    /// Brand / version about block for embedders.
    pub fn about(&self) -> RuntimeInfo {
        runtime_about()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use silicera::brand::{
        AFFILIATION_DISCLAIMER, BRAND_LINE, FUNDING_URL, HOMEPAGE, LICENSE, PHASE, REPOSITORY,
        VERSION,
    };
    use silicera::hardware::{EnvironmentSnapshot, MockHardware, SupportStatus};
    use silicera::hnep::{Confidence, HnepHeader, HnepProfile, IntegrityDigest, WorkloadEntry};
    use silicera::knowledge::{KnowledgePack, Microarch};
    use silicera::HardwareBackend;

    fn make_profile(fp: &str) -> HnepProfile {
        let header = HnepHeader {
            format: silicera::hnep::HNEP_FORMAT.into(),
            version: silicera::hnep::HNEP_VERSION,
            silicera_version: silicera::VERSION.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            fingerprint: fp.into(),
            label: "test".into(),
        };
        let environment = EnvironmentSnapshot::capture();
        let workloads = vec![WorkloadEntry {
            name: "demo".into(),
            winner: "fast".into(),
            confidence: Confidence::Medium,
            rationale: "test".into(),
            winner_median_ns: Some(90.0),
            baseline_median_ns: Some(100.0),
        }];
        let size_classes: Vec<silicera::SizeClassEntry> = Vec::new();
        let profile = HnepProfile {
            header: header.clone(),
            environment: environment.clone(),
            workloads: workloads.clone(),
            size_classes: size_classes.clone(),
            decision_tree: None,
            digest: IntegrityDigest {
                alg: "sha256".into(),
                hex: String::new(),
            },
        };
        #[derive(serde::Serialize)]
        struct Payload {
            header: HnepHeader,
            environment: EnvironmentSnapshot,
            workloads: Vec<WorkloadEntry>,
            size_classes: Vec<silicera::SizeClassEntry>,
            decision_tree: Option<silicera::specialize::DecisionTree>,
        }
        let p = Payload {
            header: profile.header.clone(),
            environment: profile.environment.clone(),
            workloads: profile.workloads.clone(),
            size_classes: profile.size_classes.clone(),
            decision_tree: None,
        };
        let bytes = serde_json::to_vec(&p).unwrap();
        let digest = IntegrityDigest::sha256(&bytes);
        HnepProfile { digest, ..profile }
    }

    #[test]
    fn mismatch_falls_back() {
        let kp = KnowledgePack::builtin();
        let host = MockHardware::zen5_dual_ccd().discover(&kp).unwrap();
        let fp = host.fingerprint.as_ref().unwrap().value.clone();
        let profile = make_profile(&fp);
        let loaded =
            LoadedProfile::from_parts(profile, host, MismatchPolicy::FallbackBaseline).unwrap();
        assert!(!loaded.force_baseline);
        assert_eq!(loaded.select_workload("demo"), "fast");
    }

    #[test]
    fn wrong_machine_baseline() {
        let kp = KnowledgePack::builtin();
        let host = MockHardware::zen5_dual_ccd().discover(&kp).unwrap();
        let mut profile = make_profile("SLC:AMD:ZEN4:19:61:00:deadbeefdeadbeef:cafebabecafebabe");
        // Fix digest for wrong fp profile
        #[derive(serde::Serialize)]
        struct Payload {
            header: HnepHeader,
            environment: EnvironmentSnapshot,
            workloads: Vec<WorkloadEntry>,
            size_classes: Vec<silicera::SizeClassEntry>,
            decision_tree: Option<silicera::specialize::DecisionTree>,
        }
        let p = Payload {
            header: profile.header.clone(),
            environment: profile.environment.clone(),
            workloads: profile.workloads.clone(),
            size_classes: profile.size_classes.clone(),
            decision_tree: None,
        };
        profile.digest = IntegrityDigest::sha256(&serde_json::to_vec(&p).unwrap());
        let loaded =
            LoadedProfile::from_parts(profile, host, MismatchPolicy::FallbackBaseline).unwrap();
        assert!(loaded.force_baseline);
        assert_eq!(loaded.select_workload("demo"), "baseline");
        assert!(loaded.status.contains("PROFILE MISMATCH"));
        assert!(loaded.status.contains(NAME));
        let _ = SupportStatus::Unsupported { reason: "x".into() };
        let _ = Microarch::Zen5;
    }

    #[test]
    fn runtime_about_has_funding() {
        let about = runtime_about();
        assert_eq!(about.name, NAME);
        assert_eq!(about.phase, PHASE);
        assert_eq!(about.version, VERSION);
        assert!(about.funding_url.contains("thanks.dev/u/gh/theworker02"));
        assert_eq!(about.brand_line, BRAND_LINE);
        assert_eq!(about.license, LICENSE);
        assert_eq!(about.homepage, HOMEPAGE);
        assert_eq!(about.repository, REPOSITORY);
        assert_eq!(about.funding_url, FUNDING_URL);
        assert_eq!(about.affiliation, AFFILIATION_DISCLAIMER);
    }

    #[test]
    fn unsupported_host_obeys_strict_policy() {
        for policy in [
            MismatchPolicy::FallbackBaseline,
            MismatchPolicy::StrictMachine,
        ] {
            let host = MockHardware::unsupported_intel()
                .discover(&KnowledgePack::builtin())
                .unwrap();
            let profile = make_profile("SLC:AMD:ZEN4:19:61:00:deadbeefdeadbeef:cafebabecafebabe");
            let result = LoadedProfile::from_parts(profile, host, policy);
            if policy == MismatchPolicy::StrictMachine {
                assert!(matches!(result, Err(SiliceraError::ProfileMismatch(_))));
            } else {
                let loaded = result.unwrap();
                assert_eq!(
                    loaded
                        .guarded_workload("demo", &["fast"], Confidence::Medium)
                        .reason,
                    DispatchReason::MachineMismatch
                );
                assert_eq!(loaded.select_size(100), "baseline");
            }
        }
    }

    #[test]
    fn direct_load_rejects_tampering_and_unknown_schema() {
        let host = MockHardware::zen5_dual_ccd()
            .discover(&KnowledgePack::builtin())
            .unwrap();
        let mut profile = make_profile(&host.fingerprint.as_ref().unwrap().value);
        profile.workloads[0].winner = "tampered".into();
        assert!(LoadedProfile::from_parts(
            profile.clone(),
            host.clone(),
            MismatchPolicy::FallbackBaseline
        )
        .is_err());
        profile.header.version = 999;
        profile.recompute_digest().unwrap();
        assert!(
            LoadedProfile::from_parts(profile, host, MismatchPolicy::FallbackBaseline).is_err()
        );
    }

    #[test]
    fn guarded_dispatch_checks_registry_confidence_and_boundaries() {
        let host = MockHardware::zen5_dual_ccd()
            .discover(&KnowledgePack::builtin())
            .unwrap();
        let mut profile = make_profile(&host.fingerprint.as_ref().unwrap().value);
        profile.decision_tree = Some(DecisionTree::from_thresholds(
            10, 20, 30, "fast", "missing", "baseline", "fast", "baseline",
        ));
        profile.recompute_digest().unwrap();
        let loaded =
            LoadedProfile::from_parts(profile, host, MismatchPolicy::StrictMachine).unwrap();
        assert_eq!(
            loaded
                .guarded_workload("demo", &["fast"], Confidence::Medium)
                .variant,
            "fast"
        );
        assert_eq!(
            loaded
                .guarded_workload("demo", &["fast"], Confidence::High)
                .reason,
            DispatchReason::InsufficientConfidence
        );
        assert_eq!(
            loaded.guarded_workload("demo", &[], Confidence::Low).reason,
            DispatchReason::UnavailableVariant
        );
        assert_eq!(
            loaded
                .guarded_workload("unknown", &["fast"], Confidence::Low)
                .reason,
            DispatchReason::MissingWorkload
        );
        assert_eq!(loaded.guarded_size(9, &["fast"]).variant, "fast");
        assert_eq!(
            loaded.guarded_size(10, &["fast"]).reason,
            DispatchReason::UnavailableVariant
        );
        assert_eq!(
            loaded.guarded_size(20, &["fast"]).reason,
            DispatchReason::ProfileBaseline
        );
        assert_eq!(loaded.guarded_size(u64::MAX, &["fast"]).variant, "fast");
    }
}
