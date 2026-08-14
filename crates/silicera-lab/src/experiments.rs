//! Experiment domains for Silicera lab demos.
//!
//! These experiments are conceptually runnable demos. Reported timings always
//! come from the measurement engine on the current host — never fabricated.

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary};
use silicera::topology::TopologyGraph;
use silicera::Result;

use crate::arms::{run_integer_arms, run_memory_arms, ArmsHarnessConfig};
use crate::bench::{CacheTarget, ConcurrencyBench, IntegerBench, MemoryBench};

/// Experiment identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentId {
    /// Multi-machine strategy split (this host = Machine A; B may be placeholder).
    SiliconSplit,
    /// Portable vs `-march=native`-style vs Silicera-selected.
    PortableVsNativeVsSilicera,
    /// Fresh machine with no prior profile (cold start).
    ColdMachine,
    /// Profile from a different fingerprint (wrong machine).
    WrongMachine,
}

impl ExperimentId {
    /// All built-in experiments.
    pub fn all() -> &'static [ExperimentId] {
        &[
            ExperimentId::SiliconSplit,
            ExperimentId::PortableVsNativeVsSilicera,
            ExperimentId::ColdMachine,
            ExperimentId::WrongMachine,
        ]
    }

    /// Stable slug.
    pub fn slug(self) -> &'static str {
        match self {
            ExperimentId::SiliconSplit => "silicon-split",
            ExperimentId::PortableVsNativeVsSilicera => "portable-vs-native-vs-silicera",
            ExperimentId::ColdMachine => "cold-machine",
            ExperimentId::WrongMachine => "wrong-machine",
        }
    }

    /// Title.
    pub fn title(self) -> &'static str {
        match self {
            ExperimentId::SiliconSplit => "Silicon Split (multi-machine protocol)",
            ExperimentId::PortableVsNativeVsSilicera => "Portable vs Native vs Silicera",
            ExperimentId::ColdMachine => "Cold Machine",
            ExperimentId::WrongMachine => "Wrong Machine",
        }
    }
}

/// One measured arm of an experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentArm {
    /// Arm name.
    pub name: String,
    /// Summary when measured.
    pub summary: Option<MeasurementSummary>,
    /// Notes (methodology / caveats).
    pub notes: String,
}

/// Experiment report — structure for real measurements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentReport {
    /// Experiment id.
    pub id: ExperimentId,
    /// Title.
    pub title: String,
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint if supported.
    pub fingerprint: Option<String>,
    /// Arms.
    pub arms: Vec<ExperimentArm>,
    /// Narrative conclusion (no invented %).
    pub conclusion: String,
}

/// Run an experiment on the given hardware/topology.
pub fn run_experiment(
    id: ExperimentId,
    info: &HardwareInfo,
    cfg: MeasurementConfig,
) -> Result<ExperimentReport> {
    match id {
        ExperimentId::SiliconSplit => silicon_split(info, cfg),
        ExperimentId::PortableVsNativeVsSilicera => portable_native_silicera(info, cfg),
        ExperimentId::ColdMachine => cold_machine(info, cfg),
        ExperimentId::WrongMachine => wrong_machine(info, cfg),
    }
}

fn silicon_split(info: &HardwareInfo, cfg: MeasurementConfig) -> Result<ExperimentReport> {
    // Local cache-crossing arms remain useful context; the research question is
    // multi-machine strategy divergence — answered via CLI silicon-split / compare.
    let eng = MeasurementEngine::new(cfg);
    let topo = &info.topology;
    let l1 = MemoryBench::for_target(topo, CacheTarget::L1);
    let dram = MemoryBench::for_target(topo, CacheTarget::Dram);
    let s_l1 = eng.measure(|| {
        let _ = l1.run();
    })?;
    let s_dram = eng.measure(|| {
        let _ = dram.run();
    })?;
    Ok(ExperimentReport {
        id: ExperimentId::SiliconSplit,
        title: ExperimentId::SiliconSplit.title().into(),
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        arms: vec![
            ExperimentArm {
                name: "Machine A L1 touch".into(),
                summary: Some(s_l1),
                notes: CacheTarget::L1.describe(topo),
            },
            ExperimentArm {
                name: "Machine A DRAM touch".into(),
                summary: Some(s_dram),
                notes: CacheTarget::Dram.describe(topo),
            },
            ExperimentArm {
                name: "Machine B".into(),
                summary: None,
                notes: "PLACEHOLDER — not measured on this run. Train on a second Zen box with \
                        `silicera silicon-split train --role B`. Never invent B numbers."
                    .into(),
            },
        ],
        conclusion: format!(
            "Question: can two machines benefit from different measured strategies? \
             Answer on this host alone: UNKNOWN (Machine B absent). \
             Local L1 vs DRAM arms show cache-level sensitivity on {} ({} domains). \
             Complete the protocol: train A → export → placeholder for B → compare when B exists. \
             See docs/research/silicon-split.md.",
            info.brand,
            topo.domain_count()
        ),
    })
}

fn portable_native_silicera(
    info: &HardwareInfo,
    cfg: MeasurementConfig,
) -> Result<ExperimentReport> {
    let arms_cfg = ArmsHarnessConfig {
        warmup: cfg.warmup,
        iterations: cfg.iterations,
        min_improvement: 0.03,
        mem_target: "L2".into(),
        mem_all_sizes: false,
    };
    let int_report = run_integer_arms(info, &arms_cfg)?;
    let mem_report = run_memory_arms(info, &arms_cfg)?;
    let mut arms = Vec::new();
    for r in [&int_report, &mem_report] {
        for a in &r.arms {
            arms.push(ExperimentArm {
                name: format!("{}:{}", r.workload, a.arm),
                summary: Some(a.summary.clone()),
                notes: a.notes.clone(),
            });
        }
    }
    let loss_note = if int_report.native_beats_silicera || mem_report.native_beats_silicera {
        " At least one workload shows native median beating Silicera dispatch — report losses."
    } else {
        ""
    };
    Ok(ExperimentReport {
        id: ExperimentId::PortableVsNativeVsSilicera,
        title: ExperimentId::PortableVsNativeVsSilicera.title().into(),
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        arms,
        conclusion: format!(
            "Identical warmup/iterations/min_improvement across arms. \
             integer: {}. memory: {}.{}",
            int_report.conclusion, mem_report.conclusion, loss_note
        ),
    })
}

fn cold_machine(info: &HardwareInfo, cfg: MeasurementConfig) -> Result<ExperimentReport> {
    let eng = MeasurementEngine::new(cfg.clone());
    let bench = IntegerBench { n: 7 };
    let cold = eng.measure(|| {
        let _ = bench.run_baseline();
    })?;
    Ok(ExperimentReport {
        id: ExperimentId::ColdMachine,
        title: ExperimentId::ColdMachine.title().into(),
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        arms: vec![ExperimentArm {
            name: "no-profile baseline".into(),
            summary: Some(cold),
            notes: format!(
                "warmup={} iterations={} — simulates first-run without HNEP",
                cfg.warmup, cfg.iterations
            ),
        }],
        conclusion: "Cold machine uses baseline dispatch until `silicera train` produces \
                     an HNEP. No speedup is claimed without a profile."
            .into(),
    })
}

fn wrong_machine(info: &HardwareInfo, cfg: MeasurementConfig) -> Result<ExperimentReport> {
    let eng = MeasurementEngine::new(cfg);
    let bench = ConcurrencyBench {
        iters: 50_000,
        threads: 4.min(info.topology.thread_count().max(1)),
    };
    let baseline = eng.measure(|| {
        let _ = bench.run_baseline();
    })?;
    Ok(ExperimentReport {
        id: ExperimentId::WrongMachine,
        title: ExperimentId::WrongMachine.title().into(),
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        arms: vec![
            ExperimentArm {
                name: "host baseline".into(),
                summary: Some(baseline),
                notes: "measured on this host".into(),
            },
            ExperimentArm {
                name: "foreign HNEP".into(),
                summary: None,
                notes: "not executed — runtime must report PROFILE MISMATCH and fall back \
                        (see silicera-runtime). Run `silicera verify` with a mismatched \
                        profile to observe the path."
                    .into(),
            },
        ],
        conclusion: "Wrong-machine safety is a correctness requirement: mismatch → baseline. \
                     Never silently apply another SKU's winners."
            .into(),
    })
}

/// Describe cache targets for docs/CLI.
pub fn describe_cache_targets(topo: &TopologyGraph) -> Vec<String> {
    [
        CacheTarget::L1,
        CacheTarget::L2,
        CacheTarget::L3,
        CacheTarget::Dram,
    ]
    .iter()
    .map(|t| t.describe(topo))
    .collect()
}

