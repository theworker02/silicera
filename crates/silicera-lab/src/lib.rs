//! # silicera-lab
//!
//! Measurement laboratory: microbenchmarks across memory, integer, float,
//! branch, and concurrency domains; cache-size workloads; tournaments; and a
//! terminal UI for live visualization of **real** measurements only.
//!
//! Numbers shown in the lab UI come from the measurement engine — never from
//! hard-coded marketing placeholders.

#![warn(missing_docs)]

pub mod arms;
pub mod bench;
pub mod calm;
pub mod experiments;
pub mod native_artifacts;
pub mod profile_gen;
pub mod silicon_split;
pub mod single_machine;
pub mod studies;
pub mod threads;
pub mod ui;

pub use arms::{
    run_concurrency_arms, run_domain_harness, run_float_arms, run_full_harness, run_integer_arms,
    run_memory_arms, ArmsHarnessConfig, ArmsReport, HarnessSuiteReport,
};
pub use bench::{
    AlignmentBench, BranchBench, CacheTarget, ConcurrencyBench, FloatBench, HostIsaDot,
    HostIsaReduce, IntegerBench, MemOpBench, MemoryBench, WorkloadKind,
};
pub use calm::{run_calm_check, CalmCheckReport};
pub use experiments::{run_experiment, ExperimentId, ExperimentReport};
pub use native_artifacts::{
    default_artifact_kernels, find_workspace_root, run_native_artifact_compare,
    run_native_artifact_suite, suite_to_workload_entries, ArtifactArm, NativeArtifactReport,
    NativeArtifactSuite,
};
pub use profile_gen::{
    generate_profile, measure_dispatch_overhead, measure_dispatch_overhead_report,
    measure_size_classes, size_class_winner_story, DispatchOverheadReport, ProfileGenConfig,
    SizeClassWinnerStory,
};
pub use silicon_split::{
    read_export, sanitize_export, split_report_paths, split_report_profiles, train_split_profile,
    write_export, write_machine_b_placeholder, SiliconSplitProtocol,
};
pub use single_machine::{
    merge_artifacts_into_profile, run_single_machine_eval, write_eval_outputs, SingleMachineEval,
};
pub use studies::{
    measure_alignment, measure_placement, spot_check_profile, AlignmentReport, AlignmentSample,
    PlacementReport, PlacementSample, PlacementStrategy, SpotCheckItem, SpotCheckReport,
};
pub use threads::{default_thread_sweep, measure_thread_count_sweep, ThreadCountReport};
pub use ui::{run_lab_ui, summary_cv, LabSessionRow, LabSessionSummary};

use silicera::measure::MeasurementConfig;
use silicera::tournament::{Tournament, TournamentConfig, TournamentResult};
use silicera::variant::{Variant, VariantId};
use silicera::Result;

/// Run a named microbenchmark tournament with two simple variants.
pub fn run_simple_tournament(
    name: &str,
    baseline: impl Fn() + Send + Sync + 'static,
    candidate: impl Fn() + Send + Sync + 'static,
    cfg: MeasurementConfig,
) -> Result<TournamentResult> {
    struct FnVariant {
        id: VariantId,
        desc: String,
        f: Box<dyn Fn() + Send + Sync>,
    }
    impl Variant for FnVariant {
        type Output = u64;
        fn id(&self) -> &VariantId {
            &self.id
        }
        fn description(&self) -> &str {
            &self.desc
        }
        fn run(&self) -> Self::Output {
            (self.f)();
            1
        }
    }

    let variants = vec![
        FnVariant {
            id: VariantId::new("baseline"),
            desc: "baseline".into(),
            f: Box::new(baseline),
        },
        FnVariant {
            id: VariantId::new("candidate"),
            desc: "candidate".into(),
            f: Box::new(candidate),
        },
    ];

    let t = Tournament::new(TournamentConfig {
        measurement: cfg,
        baseline_id: VariantId::new("baseline"),
        ..Default::default()
    });
    t.run(name, &variants)
}

