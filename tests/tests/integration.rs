//! Integration tests for Silicera Phase I.

use silicera::hardware::{HardwareBackend, MockHardware};
use silicera::hnep::HnepProfile;
use silicera::knowledge::KnowledgePack;
use silicera::specialize::{plan_size_specialization, SpecializeConfig};
use silicera_lab::profile_gen::{generate_profile, ProfileGenConfig};
use silicera_runtime::{LoadedProfile, MismatchPolicy};
use tempfile::tempdir;

#[test]
fn mock_zen5_train_and_load() {
    let kp = KnowledgePack::builtin();
    let info = MockHardware::zen5_dual_ccd().discover(&kp).unwrap();
    assert!(info.support.is_supported());

    let dir = tempdir().unwrap();
    let path = dir.path().join("t.hnep");
    let cfg = ProfileGenConfig {
        measurement: silicera::measure::MeasurementConfig {
            warmup: 1,
            iterations: 4,
            ..Default::default()
        },
        label: "itest".into(),
        size_tree: true,
        size_class_tournaments: true,
        extended_size_targets: false,
        min_improvement: 0.03,
        only: None,
    };
    let (profile, results) = generate_profile(&info, &cfg, &path).unwrap();
    assert!(!results.is_empty());
    assert!(path.exists());

    let loaded = LoadedProfile::from_parts(profile, info, MismatchPolicy::FallbackBaseline).unwrap();
    assert!(!loaded.force_baseline);
}

#[test]
fn unsupported_mock_has_no_fingerprint() {
    let kp = KnowledgePack::builtin();
    let info = MockHardware::unsupported_intel().discover(&kp).unwrap();
    assert!(!info.support.is_supported());
    assert!(info.fingerprint.is_none());
}

#[test]
fn decision_tree_plan() {
    let kp = KnowledgePack::builtin();
    let info = MockHardware::zen4_single_ccd().discover(&kp).unwrap();
    let plan = plan_size_specialization(&info.topology, &SpecializeConfig::default());
    assert!(plan.tree.l1_bytes > 0);
    let _ = HnepProfile::parse_str; // keep import used in other tests
}

#[test]
fn partial_retrain_filter_and_staleness() {
    use silicera::hardware::EnvironmentSnapshot;
    use silicera::retrain::{parse_only_filter, plan_partial_retrain, target_matches_filter};
    use silicera::staleness::{assess_staleness, DriftSeverity, StalenessPolicy};

    let kp = KnowledgePack::builtin();
    let info = MockHardware::zen5_dual_ccd().discover(&kp).unwrap();
    let dir = tempdir().unwrap();
    let path = dir.path().join("t.hnep");
    let cfg = ProfileGenConfig {
        measurement: silicera::measure::MeasurementConfig {
            warmup: 1,
            iterations: 3,
            ..Default::default()
        },
        label: "partial".into(),
        size_tree: true,
        size_class_tournaments: true,
        extended_size_targets: false,
        min_improvement: 0.03,
        only: None,
    };
    let (profile, _) = generate_profile(&info, &cfg, &path).unwrap();
    let filter = parse_only_filter("concurrency");
    assert!(target_matches_filter("concurrency", &filter));
    assert!(!target_matches_filter("float", &filter));

    let mut live = EnvironmentSnapshot::capture();
    live.logical_cpus = profile.environment.logical_cpus.saturating_add(8).max(1);
    let stale = assess_staleness(
        &profile,
        &live,
        info.fingerprint.as_ref().map(|f| f.value.as_str()),
        &StalenessPolicy::default(),
    );
    assert_eq!(stale.severity, DriftSeverity::Soft);
    assert!(stale.retrain.partial);
    let plan = plan_partial_retrain(&profile, &stale.signals);
    assert!(plan.is_partial);
}

#[test]
fn scf_and_arch_compare_mock() {
    use silicera::arch_compare::compare_mock_zen4_zen5;
    use silicera::feedback::CompilerFeedback;

    let packs = KnowledgePack::builtin();
    let report = compare_mock_zen4_zen5(&packs).unwrap();
    assert!(!report.differences.is_empty());

    let info = MockHardware::zen5_dual_ccd().discover(&packs).unwrap();
    let dir = tempdir().unwrap();
    let path = dir.path().join("t.hnep");
    let cfg = ProfileGenConfig {
        measurement: silicera::measure::MeasurementConfig {
            warmup: 1,
            iterations: 3,
            ..Default::default()
        },
        label: "scf".into(),
        size_tree: true,
        size_class_tournaments: false,
        extended_size_targets: false,
        min_improvement: 0.03,
        only: Some("integer".into()),
    };
    let (profile, _) = generate_profile(&info, &cfg, &path).unwrap();
    let scf = CompilerFeedback::from_hnep(&profile).unwrap();
    assert_eq!(scf.format, "silicera-compiler-feedback");
    assert!(!scf.fingerprint.is_empty());
}

#[test]
fn knowledge_packs_load_from_dir() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("profiles")
        .join("amd");
    let kp = KnowledgePack::load_dir(&root).unwrap();
    assert!(kp.packs.len() >= 3);
}
