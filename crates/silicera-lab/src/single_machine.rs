//! Single-Zen evaluation pack — everything one physical machine can prove.
//!
//! Intentionally excludes Silicon Split Machine B. Produces a JSON report plus
//! optional repro pack for AMD-facing inspection.

use std::path::Path;

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::hnep::HnepProfile;
use silicera::measure::MeasurementConfig;
use silicera::repro::ReproPack;
use silicera::Result;

use crate::measure_alignment;
use crate::arms::{run_domain_harness, ArmsHarnessConfig, HarnessSuiteReport};
use crate::calm::{run_calm_check, CalmCheckReport};
use crate::native_artifacts::{
    default_artifact_kernels, run_native_artifact_suite, suite_to_workload_entries,
    NativeArtifactSuite,
};
use crate::measure_placement;
use crate::threads::measure_thread_count_sweep;

/// Full single-machine evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleMachineEval {
    /// Format id.
    pub format: String,
    /// Schema version.
    pub version: u32,
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Silicera version.
    pub silicera_version: String,
    /// Calm check.
    pub calm: CalmCheckReport,
    /// Harness suite (all domains).
    pub harness: HarnessSuiteReport,
    /// Native artifact suite.
    pub artifacts: NativeArtifactSuite,
    /// Alignment recommendation.
    pub alignment_matters: bool,
    /// Alignment one-liner.
    pub alignment_note: String,
    /// Placement fastest strategy.
    pub placement_fastest: String,
    /// Thread sweep best count.
    pub best_thread_count: usize,
    /// Thread sweep note.
    pub threads_note: String,
    /// Honest research boundary.
    pub deferred: Vec<String>,
    /// Summary for humans.
    pub summary: String,
}

/// Run the single-Zen evaluation pack.
pub fn run_single_machine_eval(
    info: &HardwareInfo,
    workspace: &Path,
    harness_iters: usize,
    artifact_iters: usize,
) -> Result<SingleMachineEval> {
    let calm = run_calm_check(info, 24)?;
    let harness = run_domain_harness(
        info,
        &ArmsHarnessConfig {
            warmup: 3,
            iterations: harness_iters,
            min_improvement: 0.03,
            mem_target: "L2".into(),
            mem_all_sizes: true,
        },
        "all",
    )?;
    let artifacts = run_native_artifact_suite(
        info,
        default_artifact_kernels(),
        artifact_iters,
        0.03,
        workspace,
    )?;
    let align = measure_alignment(
        info,
        65_536,
        &MeasurementConfig {
            warmup: 2,
            iterations: 12,
            ..Default::default()
        },
        0.03,
    )?;
    let placement = measure_placement(
        info,
        4,
        40_000,
        &MeasurementConfig {
            warmup: 2,
            iterations: 10,
            ..Default::default()
        },
    )?;
    let threads = measure_thread_count_sweep(
        info,
        &MeasurementConfig {
            warmup: 2,
            iterations: 10,
            ..Default::default()
        },
        80_000,
        None,
    )?;

    let summary = format!(
        "single-zen eval on {}: calm={} harness_portable_wins={} artifact_native_wins={} align_matters={} placement={} best_threads={}",
        info.brand,
        calm.calm,
        harness.portable_win_count,
        artifacts.native_win_count,
        align.alignment_matters,
        placement.fastest,
        threads.best_candidate_threads
    );

    Ok(SingleMachineEval {
        format: "silicera-single-machine-eval".into(),
        version: 1,
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        silicera_version: silicera::VERSION.into(),
        calm,
        harness,
        artifacts,
        alignment_matters: align.alignment_matters,
        alignment_note: align.recommendation,
        placement_fastest: placement.fastest,
        best_thread_count: threads.best_candidate_threads,
        threads_note: threads.conclusion.clone(),
        deferred: vec![
            "Silicon Split Machine B / cross-machine YES-NO (requires second Zen host)".into(),
            "Full PGO/BOLT measured tables (optional external toolchains)".into(),
        ],
        summary,
    })
}

/// Merge artifact suite winners into an existing HNEP and rewrite it.
pub fn merge_artifacts_into_profile(
    profile_path: &Path,
    suite: &NativeArtifactSuite,
) -> Result<HnepProfile> {
    let mut profile = HnepProfile::read_from(profile_path)?;
    for entry in suite_to_workload_entries(suite) {
        profile.upsert_workload(entry)?;
    }
    profile.write_to(profile_path)?;
    Ok(profile)
}

/// Write eval JSON + repro pack beside it.
pub fn write_eval_outputs(
    info: &HardwareInfo,
    eval: &SingleMachineEval,
    out_json: &Path,
) -> Result<()> {
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out_json, serde_json::to_string_pretty(eval)?)?;
    let repro = ReproPack::capture(
        info,
        "single-machine-eval",
        serde_json::json!({
            "output": out_json.display().to_string(),
            "format": eval.format,
            "version": eval.version,
        }),
        None,
        None,
    );
    let repro_path = out_json.with_extension("repro.json");
    repro.write_to(&repro_path)?;
    Ok(())
}
