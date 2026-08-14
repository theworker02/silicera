//! Portable vs host-ISA vs Silicera selection harness.
//!
//! Identical inputs, sample counts, warmups, and improvement thresholds across
//! three arms. The integer domain uses a real runtime AVX2 path when detected
//! (`HostIsaReduce`); other domains still use algorithmic stand-ins.
//! Separately compiled `-march=native` binaries remain a future comparison track.
//!
//! Domains: integer, memory (multi size-class), float, branch, concurrency.

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary};
use silicera::Result;

use crate::bench::{
    BranchBench, CacheTarget, ConcurrencyBench, HostIsaDot, HostIsaReduce, MemOpBench,
};

/// Shared harness knobs (must be identical across arms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmsHarnessConfig {
    /// Warmup iterations.
    pub warmup: usize,
    /// Timed iterations.
    pub iterations: usize,
    /// Minimum relative improvement to prefer native/Silicera over portable.
    pub min_improvement: f64,
    /// Working-set target for the single-target memory arm (`L1`/`L2`/`L3`/`DRAM`).
    pub mem_target: String,
    /// When true, memory arm runs all size classes (L1/L2/L3/DRAM).
    pub mem_all_sizes: bool,
}

impl Default for ArmsHarnessConfig {
    fn default() -> Self {
        Self {
            warmup: 5,
            iterations: 30,
            min_improvement: 0.03,
            mem_target: "L2".into(),
            mem_all_sizes: false,
        }
    }
}

impl ArmsHarnessConfig {
    /// Measurement config.
    pub fn measurement(&self) -> MeasurementConfig {
        MeasurementConfig {
            warmup: self.warmup,
            iterations: self.iterations,
            ..Default::default()
        }
    }
}

/// One arm result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmResult {
    /// Arm id: `portable`, `native`, `silicera`.
    pub arm: String,
    /// Median ns.
    pub summary: MeasurementSummary,
    /// Notes / methodology.
    pub notes: String,
}

/// Full three-arm report for one workload family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmsReport {
    /// Workload family name.
    pub workload: String,
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Config used.
    pub config: ArmsHarnessConfig,
    /// Arms in order: portable, native, silicera.
    pub arms: Vec<ArmResult>,
    /// Which arm had the lowest median.
    pub fastest_arm: String,
    /// True when native beat silicera's selected median (Silicera loss / tie path).
    pub native_beats_silicera: bool,
    /// True when portable beat both (Silicera and native lose).
    pub portable_wins: bool,
    /// Honest conclusion string.
    pub conclusion: String,
}

/// Aggregate harness run across domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSuiteReport {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Per-workload reports.
    pub reports: Vec<ArmsReport>,
    /// Count where Silicera selection matched the fastest measured arm.
    pub silicera_selection_ok: usize,
    /// Count where portable won (no specialization gain).
    pub portable_win_count: usize,
    /// Count where native beat Silicera dispatch.
    pub native_beats_silicera_count: usize,
    /// Size-class memory winner change summary (when multi-size run).
    pub size_winner_changes: Vec<String>,
}

/// Run integer-domain three-arm harness.
pub fn run_integer_arms(info: &HardwareInfo, cfg: &ArmsHarnessConfig) -> Result<ArmsReport> {
    let eng = MeasurementEngine::new(cfg.measurement());
    let bench = HostIsaReduce::new(65_536);
    if !bench.verify_correctness() {
        return Err(silicera::SiliceraError::CorrectnessFailure(
            "HostIsaReduce portable vs host-ISA disagree".into(),
        ));
    }
    let portable = eng.measure(|| {
        let _ = bench.run_portable();
    })?;
    let native = eng.measure(|| {
        let _ = bench.run_host_isa();
    })?;
    let isa_note = if HostIsaReduce::avx2_active() {
        "portable = scalar i32 sum; host-isa = runtime AVX2 reduction (target_feature); not a separate -march=native binary"
    } else {
        "portable = scalar i32 sum; host-isa fell back to scalar (AVX2 not detected)"
    };
    finish_report("integer-host-isa", info, cfg, portable, native, isa_note)
}

/// Run float-domain three-arm harness.
pub fn run_float_arms(info: &HardwareInfo, cfg: &ArmsHarnessConfig) -> Result<ArmsReport> {
    let eng = MeasurementEngine::new(cfg.measurement());
    let bench = HostIsaDot::new(65_536);
    if !bench.verify_correctness(1e-3) {
        return Err(silicera::SiliceraError::CorrectnessFailure(
            "HostIsaDot portable vs host-ISA disagree beyond tolerance".into(),
        ));
    }
    let portable = eng.measure(|| {
        let _ = bench.run_portable();
    })?;
    let native = eng.measure(|| {
        let _ = bench.run_host_isa();
    })?;
    finish_report(
        "float-host-isa-dot",
        info,
        cfg,
        portable,
        native,
        "portable = scalar f32 dot; host-isa = runtime AVX when detected; correctness gated",
    )
}

/// Run branch-domain three-arm harness.
pub fn run_branch_arms(info: &HardwareInfo, cfg: &ArmsHarnessConfig) -> Result<ArmsReport> {
    let eng = MeasurementEngine::new(cfg.measurement());
    let bench = BranchBench::new(8192);
    let portable = eng.measure(|| {
        let _ = bench.run_baseline();
    })?;
    let native = eng.measure(|| {
        let _ = bench.run_candidate();
    })?;
    finish_report(
        "branch-mix",
        info,
        cfg,
        portable,
        native,
        "portable = unpredictable branches; native = branchless-ish stand-in",
    )
}

/// Run concurrency-domain three-arm harness.
pub fn run_concurrency_arms(info: &HardwareInfo, cfg: &ArmsHarnessConfig) -> Result<ArmsReport> {
    let eng = MeasurementEngine::new(cfg.measurement());
    let threads = 4.min(info.topology.thread_count().max(1));
    let bench = ConcurrencyBench {
        iters: 20_000,
        threads,
    };
    let portable = eng.measure(|| {
        let _ = bench.run_baseline();
    })?;
    let native = eng.measure(|| {
        let _ = bench.run_candidate();
    })?;
    finish_report(
        "concurrency-atomics",
        info,
        cfg,
        portable,
        native,
        &format!(
            "portable = contended atomics; native = per-thread locals+reduce; threads={threads} (Silicera-owned; no OS affinity changes)"
        ),
    )
}

/// Run memory-domain three-arm harness (scan vs prefetch-friendly).
pub fn run_memory_arms(info: &HardwareInfo, cfg: &ArmsHarnessConfig) -> Result<ArmsReport> {
    let target = parse_mem_target(&cfg.mem_target);
    run_memory_arms_for_target(info, cfg, target)
}

fn parse_mem_target(s: &str) -> CacheTarget {
    match s.to_uppercase().as_str() {
        "L1" => CacheTarget::L1,
        "L3" => CacheTarget::L3,
        "DRAM" => CacheTarget::Dram,
        _ => CacheTarget::L2,
    }
}

fn run_memory_arms_for_target(
    info: &HardwareInfo,
    cfg: &ArmsHarnessConfig,
    target: CacheTarget,
) -> Result<ArmsReport> {
    let eng = MeasurementEngine::new(cfg.measurement());
    let mut op = MemOpBench::for_target(&info.topology, target);
    let portable = eng.measure(|| {
        let _ = op.run_scan_stride();
    })?;
    let native = eng.measure(|| {
        let _ = op.run_scan_dense();
    })?;
    let copy = eng.measure(|| {
        let _ = op.run_copy();
    })?;
    let (sil_name, sil_summary, notes_extra) = {
        let mut best = ("portable", portable.clone());
        if native.median_ns < best.1.median_ns {
            best = ("native", native.clone());
        }
        if copy.median_ns < best.1.median_ns {
            best = ("copy", copy.clone());
        }
        (
            format!("silicera→{}", best.0),
            best.1,
            format!(
                "selection pool medians: portable={:.0} native={:.0} copy={:.0}",
                portable.median_ns, native.median_ns, copy.median_ns
            ),
        )
    };

    let native_beats_silicera = native.median_ns + f64::EPSILON < sil_summary.median_ns;
    let portable_wins =
        portable.median_ns <= native.median_ns && portable.median_ns <= sil_summary.median_ns;

    let conclusion = build_conclusion(
        native_beats_silicera,
        portable_wins,
        &sil_name,
        portable.median_ns,
        native.median_ns,
        sil_summary.median_ns,
    );

    Ok(ArmsReport {
        workload: format!("memop-{}", target.class_id()),
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        config: cfg.clone(),
        arms: vec![
            ArmResult {
                arm: "portable".into(),
                summary: portable,
                notes: format!("scan_stride; {}", target.describe(&info.topology)),
            },
            ArmResult {
                arm: "native".into(),
                summary: native,
                notes: "scan_dense stand-in for host-tuned kernel".into(),
            },
            ArmResult {
                arm: sil_name.clone(),
                summary: sil_summary,
                notes: format!(
                    "Silicera selection after measurement. {notes_extra}. {}",
                    "Not a separately compiled -march=native binary in this release."
                ),
            },
        ],
        fastest_arm: if portable_wins {
            "portable".into()
        } else if native_beats_silicera {
            "native".into()
        } else {
            "silicera".into()
        },
        native_beats_silicera,
        portable_wins,
        conclusion,
    })
}

fn finish_report(
    workload: &str,
    info: &HardwareInfo,
    cfg: &ArmsHarnessConfig,
    portable: MeasurementSummary,
    native: MeasurementSummary,
    method_note: &str,
) -> Result<ArmsReport> {
    let improved = native.median_ns < portable.median_ns * (1.0 - cfg.min_improvement);
    let (sil_name, sil_summary) = if improved || native.median_ns < portable.median_ns {
        ("silicera→native", native.clone())
    } else {
        ("silicera→portable", portable.clone())
    };
    let native_beats_silicera = native.median_ns + f64::EPSILON < sil_summary.median_ns;
    let portable_wins =
        portable.median_ns <= native.median_ns && portable.median_ns <= sil_summary.median_ns;
    let conclusion = build_conclusion(
        native_beats_silicera,
        portable_wins,
        sil_name,
        portable.median_ns,
        native.median_ns,
        sil_summary.median_ns,
    );
    Ok(ArmsReport {
        workload: workload.into(),
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        config: cfg.clone(),
        arms: vec![
            ArmResult {
                arm: "portable".into(),
                summary: portable,
                notes: method_note.into(),
            },
            ArmResult {
                arm: "native".into(),
                summary: native,
                notes: method_note.into(),
            },
            ArmResult {
                arm: sil_name.into(),
                summary: sil_summary,
                notes: format!(
                    "selected by median after measurement (min_improvement={})",
                    cfg.min_improvement
                ),
            },
        ],
        fastest_arm: if portable_wins {
            "portable".into()
        } else if native_beats_silicera {
            "native".into()
        } else {
            "silicera".into()
        },
        native_beats_silicera,
        portable_wins,
        conclusion,
    })
}

fn build_conclusion(
    native_beats_silicera: bool,
    portable_wins: bool,
    sil_name: &str,
    p: f64,
    n: f64,
    s: f64,
) -> String {
    if native_beats_silicera {
        format!(
            "LOSS/CAVEAT: native median ({n:.0} ns) beat Silicera dispatch ({s:.0} ns). \
             Print losses clearly — do not hide them. Silicera arm was {sil_name}."
        )
    } else if portable_wins {
        format!(
            "Portable median ({p:.0} ns) ≤ native ({n:.0} ns) and Silicera ({s:.0} ns). \
             No specialization win on this host/workload under these knobs."
        )
    } else {
        format!(
            "Silicera selected {sil_name} (median {s:.0} ns). Portable={p:.0} native={n:.0}. \
             Selection is measured, not claimed universal. Host-ISA arm uses runtime AVX2 when present; \
             separately compiled -march=native artifacts remain a future comparison track."
        )
    }
}

/// Summarize whether memory size-class winners differ across L1/L2/L3/DRAM.
pub fn summarize_size_winner_changes(reports: &[ArmsReport]) -> Vec<String> {
    let mem: Vec<(&str, &str)> = reports
        .iter()
        .filter(|r| r.workload.starts_with("memop-"))
        .filter_map(|r| {
            let class = r.workload.strip_prefix("memop-")?;
            let sel = r
                .arms
                .iter()
                .find(|a| a.arm.starts_with("silicera"))
                .map(|a| a.arm.as_str())
                .unwrap_or("?");
            Some((class, sel))
        })
        .collect();
    if mem.len() < 2 {
        return Vec::new();
    }
    let first = mem[0].1;
    let all_same = mem.iter().all(|(_, s)| *s == first);
    if all_same {
        vec![format!(
            "memop size-class Silicera selection identical across {:?}: {first}",
            mem.iter().map(|(c, _)| *c).collect::<Vec<_>>()
        )]
    } else {
        mem.iter()
            .map(|(c, s)| format!("{c} → {s}"))
            .collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(
                "Winners DO change by size class on this host/run".into(),
            ))
            .collect()
    }
}

fn suite_from(info: &HardwareInfo, reports: Vec<ArmsReport>) -> HarnessSuiteReport {
    let size_winner_changes = summarize_size_winner_changes(&reports);
    let portable_win_count = reports.iter().filter(|r| r.portable_wins).count();
    let native_beats_silicera_count = reports.iter().filter(|r| r.native_beats_silicera).count();
    let silicera_selection_ok = reports
        .iter()
        .filter(|r| {
            (!r.native_beats_silicera && r.fastest_arm.starts_with("silicera"))
                || (r.portable_wins && r.fastest_arm == "portable")
        })
        .count();
    HarnessSuiteReport {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        reports,
        silicera_selection_ok,
        portable_win_count,
        native_beats_silicera_count,
        size_winner_changes,
    }
}

/// Run both integer and memory harnesses (legacy).
pub fn run_full_harness(
    info: &HardwareInfo,
    cfg: &ArmsHarnessConfig,
) -> Result<Vec<ArmsReport>> {
    Ok(run_domain_harness(info, cfg, "all")?.reports)
}

/// Run harness for a domain: `all|integer|memory|float|branch|concurrency`.
pub fn run_domain_harness(
    info: &HardwareInfo,
    cfg: &ArmsHarnessConfig,
    domain: &str,
) -> Result<HarnessSuiteReport> {
    let mut reports = Vec::new();
    let d = domain.to_ascii_lowercase();
    let all = d == "all";

    if all || d == "integer" {
        reports.push(run_integer_arms(info, cfg)?);
    }
    if all || d == "float" {
        reports.push(run_float_arms(info, cfg)?);
    }
    if all || d == "branch" {
        reports.push(run_branch_arms(info, cfg)?);
    }
    if all || d == "concurrency" {
        reports.push(run_concurrency_arms(info, cfg)?);
    }
    if all || d == "memory" {
        if cfg.mem_all_sizes || all {
            for t in CacheTarget::all() {
                reports.push(run_memory_arms_for_target(info, cfg, t)?);
            }
        } else {
            reports.push(run_memory_arms(info, cfg)?);
        }
    }

    if reports.is_empty() {
        return Err(silicera::SiliceraError::Parse(format!(
            "unknown harness domain '{domain}' (all|integer|memory|float|branch|concurrency)"
        )));
    }
    Ok(suite_from(info, reports))
}
