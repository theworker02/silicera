//! Silicon Split — multi-machine measured strategy comparison.
//!
//! Protocol: identical workload set + candidate set + measurement knobs on
//! Machine A and Machine B. Artifacts are comparable HNEPs / sanitized exports.
//! Until Machine B exists, emit a **placeholder schema** and verdict UNKNOWN.
//! Never invent Machine B numbers.

use std::path::Path;

use serde::{Deserialize, Serialize};
use silicera::compare::{
    compare_profiles, compare_with_placeholder, CompareReport, MachineBPlaceholder,
    MeasurementEcho, SanitizedSplitExport, StrategyVector, SILICON_SPLIT_PROTOCOL,
    SILICON_SPLIT_PROTOCOL_VERSION,
};
use silicera::hardware::HardwareInfo;
use silicera::hnep::HnepProfile;
use silicera::measure::MeasurementConfig;
use silicera::Result;

use crate::profile_gen::{generate_profile, ProfileGenConfig};

/// Fixed protocol knobs for Silicon Split (must match across machines).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiliconSplitProtocol {
    /// Protocol id.
    pub protocol: String,
    /// Protocol semver.
    pub protocol_version: String,
    /// Measurement echo.
    pub measurement: MeasurementEcho,
}

impl Default for SiliconSplitProtocol {
    fn default() -> Self {
        Self {
            protocol: SILICON_SPLIT_PROTOCOL.into(),
            protocol_version: SILICON_SPLIT_PROTOCOL_VERSION.into(),
            measurement: MeasurementEcho {
                warmup: 3,
                iterations: 20,
                min_improvement: 0.03,
                candidate_set: "baseline|scan_dense|copy|copy_loop|candidate".into(),
                workload_set: "memory-l2|integer|float|branch|memscan-l1|memscan-l2|memscan-l3|memscan-dram"
                    .into(),
            },
        }
    }
}

impl SiliconSplitProtocol {
    /// Build profile-gen config locked to this protocol.
    pub fn profile_gen_config(&self, role: &str) -> ProfileGenConfig {
        ProfileGenConfig {
            measurement: MeasurementConfig {
                warmup: self.measurement.warmup,
                iterations: self.measurement.iterations,
                ..Default::default()
            },
            label: format!("silicon-split-{role}"),
            size_tree: true,
            size_class_tournaments: true,
            extended_size_targets: false,
            min_improvement: self.measurement.min_improvement,
            only: None,
        }
    }
}

/// Train Machine A or B under the Silicon Split protocol.
pub fn train_split_profile(
    info: &HardwareInfo,
    role: &str,
    out: &Path,
    protocol: &SiliconSplitProtocol,
) -> Result<(HnepProfile, SanitizedSplitExport)> {
    let cfg = protocol.profile_gen_config(role);
    let (profile, _results) = generate_profile(info, &cfg, out)?;
    let export = sanitize_export(&profile, role, &protocol.measurement);
    Ok((profile, export))
}

/// Wrap an existing HNEP as a sanitized Silicon Split export.
pub fn sanitize_export(
    profile: &HnepProfile,
    role: &str,
    measurement: &MeasurementEcho,
) -> SanitizedSplitExport {
    SanitizedSplitExport {
        protocol: SILICON_SPLIT_PROTOCOL.into(),
        protocol_version: SILICON_SPLIT_PROTOCOL_VERSION.into(),
        machine_role: role.into(),
        strategy_vector: StrategyVector::from_profile(role, profile),
        measurement_echo: measurement.clone(),
        profile: profile.clone(),
    }
}

/// Write sanitized export JSON.
pub fn write_export(export: &SanitizedSplitExport, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(export)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Read sanitized export JSON.
pub fn read_export(path: &Path) -> Result<SanitizedSplitExport> {
    let text = std::fs::read_to_string(path)?;
    let export: SanitizedSplitExport = serde_json::from_str(&text)?;
    export.profile.verify_integrity()?;
    if export.protocol != SILICON_SPLIT_PROTOCOL {
        return Err(silicera::SiliceraError::Parse(format!(
            "unexpected silicon-split protocol {}",
            export.protocol
        )));
    }
    Ok(export)
}

/// Write Machine B placeholder next to Machine A artifacts.
pub fn write_machine_b_placeholder(a: &HnepProfile, path: &Path) -> Result<MachineBPlaceholder> {
    let ph = MachineBPlaceholder::from_machine_a(a);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&ph)?)?;
    Ok(ph)
}

/// Produce a Silicon Split report from two profiles, or A + placeholder.
pub fn split_report_profiles(a: &HnepProfile, b: Option<&HnepProfile>) -> CompareReport {
    match b {
        Some(b) => compare_profiles(a, b),
        None => compare_with_placeholder(a, "second-zen-box"),
    }
}

/// Load exports and compare (B path optional → placeholder verdict).
pub fn split_report_paths(a: &Path, b: Option<&Path>) -> Result<CompareReport> {
    let pa = if a.extension().and_then(|e| e.to_str()) == Some("json") {
        read_export(a)?.profile
    } else {
        HnepProfile::read_from(a)?
    };
    let pb = match b {
        Some(path) => {
            let p = if path.extension().and_then(|e| e.to_str()) == Some("json") {
                read_export(path)?.profile
            } else {
                HnepProfile::read_from(path)?
            };
            Some(p)
        }
        None => None,
    };
    Ok(split_report_profiles(&pa, pb.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use silicera::hardware::{HardwareBackend, MockHardware};
    use silicera::knowledge::KnowledgePack;
    use silicera::SplitVerdict;
    use tempfile::tempdir;

    #[test]
    fn mock_a_placeholder_unknown() {
        let kp = KnowledgePack::builtin();
        let info = MockHardware::zen5_dual_ccd().discover(&kp).unwrap();
        let dir = tempdir().unwrap();
        let out = dir.path().join("a.hnep");
        let protocol = SiliconSplitProtocol {
            measurement: MeasurementEcho {
                warmup: 1,
                iterations: 3,
                min_improvement: 0.03,
                ..MeasurementEcho::default()
            },
            ..Default::default()
        };
        let (profile, export) = train_split_profile(&info, "A", &out, &protocol).unwrap();
        assert!(!export.strategy_vector.workloads.is_empty());
        let report = split_report_profiles(&profile, None);
        assert_eq!(report.verdict, SplitVerdict::Unknown);
        assert!(report.b_is_placeholder);
    }

    #[test]
    fn mock_two_machines_compare_no_invented_physics() {
        // Two mock SKUs: compare structure only — timings are mock-host measurements,
        // not fabricated second-SKU marketing numbers.
        let kp = KnowledgePack::builtin();
        let a_info = MockHardware::zen5_dual_ccd().discover(&kp).unwrap();
        let b_info = MockHardware::zen4_single_ccd().discover(&kp).unwrap();
        let dir = tempdir().unwrap();
        let protocol = SiliconSplitProtocol {
            measurement: MeasurementEcho {
                warmup: 1,
                iterations: 3,
                min_improvement: 0.03,
                ..MeasurementEcho::default()
            },
            ..Default::default()
        };
        let (pa, _) = train_split_profile(&a_info, "A", &dir.path().join("a.hnep"), &protocol).unwrap();
        let (pb, _) = train_split_profile(&b_info, "B", &dir.path().join("b.hnep"), &protocol).unwrap();
        let report = compare_profiles(&pa, &pb);
        assert!(!report.b_is_placeholder);
        assert!(!report.fingerprint_a.is_empty());
        assert!(!report.fingerprint_b.is_empty());
        assert_ne!(report.fingerprint_a, report.fingerprint_b);
        // Verdict is whatever measurement says — must be one of the real enums.
        let _ = report.verdict.label();
    }
}

