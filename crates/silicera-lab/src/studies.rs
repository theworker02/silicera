//! Research studies: alignment, placement, and spot-check verification.
//!
//! Consolidated from former `alignment`, `placement`, and `spot_check` modules.
//! Public types and functions remain re-exported at the crate root.

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::hnep::{Confidence, HnepProfile};
use silicera::measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary};
use silicera::Result;

use crate::bench::{AlignmentBench, CacheTarget, ConcurrencyBench, FloatBench, IntegerBench, MemOpBench};

// ── Alignment ───────────────────────────────────────────────────────────────

/// One alignment offset sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentSample {
    /// Byte offset into a larger buffer.
    pub offset: usize,
    /// Scalar checksum median ns.
    pub scalar: MeasurementSummary,
    /// Chunked checksum median ns.
    pub chunked: MeasurementSummary,
    /// Winner name for this offset.
    pub winner: String,
    /// Relative improvement of winner vs loser (0 if tie/no meaningful gain).
    pub improvement: f64,
}

/// Full alignment report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentReport {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Working-set length.
    pub len: usize,
    /// Samples per offset.
    pub samples: Vec<AlignmentSample>,
    /// True when winners differ across offsets.
    pub alignment_matters: bool,
    /// Recommendation for dispatch complexity.
    pub recommendation: String,
}

/// Run alignment experiment for offsets `[0, 1, 7, 15, 63]`.
pub fn measure_alignment(
    info: &HardwareInfo,
    len: usize,
    cfg: &MeasurementConfig,
    min_improvement: f64,
) -> Result<AlignmentReport> {
    let eng = MeasurementEngine::new(cfg.clone());
    let offsets = [0usize, 1, 7, 15, 63];
    let mut samples = Vec::new();
    for &offset in &offsets {
        let b = AlignmentBench::new(len, offset);
        let scalar = eng.measure(|| {
            let _ = b.run_scalar();
        })?;
        let chunked = eng.measure(|| {
            let _ = b.run_chunked();
        })?;
        let (winner, improvement) = if chunked.median_ns < scalar.median_ns * (1.0 - min_improvement)
        {
            (
                "chunked".into(),
                (scalar.median_ns - chunked.median_ns) / scalar.median_ns,
            )
        } else if scalar.median_ns < chunked.median_ns * (1.0 - min_improvement) {
            (
                "scalar".into(),
                (chunked.median_ns - scalar.median_ns) / chunked.median_ns,
            )
        } else {
            ("no_meaningful_difference".into(), 0.0)
        };
        samples.push(AlignmentSample {
            offset,
            scalar,
            chunked,
            winner,
            improvement,
        });
    }
    let meaningful: Vec<&str> = samples
        .iter()
        .filter(|s| s.winner != "no_meaningful_difference")
        .map(|s| s.winner.as_str())
        .collect();
    let alignment_matters = meaningful.len() >= 2
        && meaningful.windows(2).any(|w| w[0] != w[1])
        || samples.iter().any(|s| {
            s.offset != 0
                && s.winner != "no_meaningful_difference"
                && samples[0].winner != s.winner
                && samples[0].winner != "no_meaningful_difference"
        });
    // Also treat large absolute timing shifts at misalignment as "matters" for docs.
    let base = samples[0].scalar.median_ns;
    let timing_shift = samples.iter().any(|s| {
        s.offset != 0 && (s.scalar.median_ns - base).abs() / base.max(1.0) > 0.10
    });
    let alignment_matters = alignment_matters || timing_shift;
    let recommendation = if alignment_matters {
        "Alignment materially affected measurements on this host — consider alignment in dispatch for this workload class.".into()
    } else {
        "No meaningful alignment-driven strategy change under these knobs — do not add dispatch complexity.".into()
    };
    Ok(AlignmentReport {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        len,
        samples,
        alignment_matters,
        recommendation,
    })
}

// ── Placement ───────────────────────────────────────────────────────────────

/// Placement strategy under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementStrategy {
    /// OS default — no affinity.
    OsDefault,
    /// Prefer first N logical CPUs (compact).
    Compact,
    /// Spread across even logical CPUs (sparse).
    Spread,
}

impl PlacementStrategy {
    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            PlacementStrategy::OsDefault => "os_default",
            PlacementStrategy::Compact => "compact",
            PlacementStrategy::Spread => "spread",
        }
    }
}

/// One placement measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementSample {
    /// Strategy.
    pub strategy: String,
    /// Thread count.
    pub threads: usize,
    /// Summary.
    pub summary: MeasurementSummary,
    /// Whether affinity was applied.
    pub affinity_applied: bool,
    /// Notes.
    pub notes: String,
}

/// Placement sweep report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementReport {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Logical CPU count.
    pub logical_cpus: usize,
    /// Samples.
    pub samples: Vec<PlacementSample>,
    /// Fastest strategy label.
    pub fastest: String,
    /// Conclusion.
    pub conclusion: String,
}

/// Run a constant-work parallel checksum under different placements.
pub fn measure_placement(
    info: &HardwareInfo,
    threads: usize,
    iters_per_thread: u64,
    cfg: &MeasurementConfig,
) -> Result<PlacementReport> {
    let eng = MeasurementEngine::new(cfg.clone());
    let logical = info.topology.thread_count().max(1);
    let threads = threads.clamp(1, logical);
    let strategies = [
        PlacementStrategy::OsDefault,
        PlacementStrategy::Compact,
        PlacementStrategy::Spread,
    ];
    let mut samples = Vec::new();
    for strat in strategies {
        let (masks, notes) = affinity_masks(strat, threads, logical);
        let summary = eng.measure(|| {
            run_placed(threads, iters_per_thread, &masks);
        })?;
        samples.push(PlacementSample {
            strategy: strat.label().into(),
            threads,
            summary,
            affinity_applied: masks.iter().any(|m| m.is_some()),
            notes,
        });
    }

    let mut fastest = "os_default".to_string();
    let mut best = f64::INFINITY;
    for s in &samples {
        if s.summary.median_ns < best {
            best = s.summary.median_ns;
            fastest = s.strategy.clone();
        }
    }

    let conclusion = format!(
        "Fastest under these knobs: {fastest}. Placement changes global OS policy: no. \
         Affinity is Silicera-owned and temporary. Do not assume compact > spread."
    );
    Ok(PlacementReport {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        logical_cpus: logical,
        samples,
        fastest,
        conclusion,
    })
}

fn affinity_masks(
    strat: PlacementStrategy,
    threads: usize,
    logical: usize,
) -> (Vec<Option<usize>>, String) {
    match strat {
        PlacementStrategy::OsDefault => (
            vec![None; threads],
            "no affinity; OS schedules freely".into(),
        ),
        PlacementStrategy::Compact => {
            let masks: Vec<Option<usize>> = (0..threads).map(|i| Some(i % logical)).collect();
            (
                masks,
                format!("compact: logical CPUs 0..{}", threads.min(logical)),
            )
        }
        PlacementStrategy::Spread => {
            let step = (logical / threads.max(1)).max(1);
            let masks: Vec<Option<usize>> = (0..threads)
                .map(|i| Some((i * step) % logical))
                .collect();
            (
                masks,
                format!("spread: step={step} across {logical} logical CPUs"),
            )
        }
    }
}

fn run_placed(threads: usize, iters: u64, masks: &[Option<usize>]) {
    let mut handles = Vec::new();
    for t in 0..threads {
        let mask = masks.get(t).copied().flatten();
        handles.push(std::thread::spawn(move || {
            if let Some(cpu) = mask {
                set_current_thread_affinity(cpu);
            }
            let mut x = t as u64;
            for i in 0..iters {
                x = x
                    .wrapping_mul(1664525)
                    .wrapping_add(1013904223)
                    .wrapping_add(i);
            }
            std::hint::black_box(x)
        }));
    }
    for h in handles {
        let _ = h.join();
    }
}

#[cfg(windows)]
mod win_affinity {
    use std::ffi::c_void;

    type Handle = *mut c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThread() -> Handle;
        fn SetThreadAffinityMask(thread: Handle, mask: usize) -> usize;
    }

    pub fn set(logical_cpu: usize) {
        if logical_cpu < usize::BITS as usize {
            let mask = 1usize << logical_cpu;
            // SAFETY: Silicera-owned thread; single-bit affinity mask.
            unsafe {
                let _ = SetThreadAffinityMask(GetCurrentThread(), mask);
            }
        }
    }
}

fn set_current_thread_affinity(logical_cpu: usize) {
    #[cfg(windows)]
    {
        win_affinity::set(logical_cpu);
    }
    #[cfg(not(windows))]
    {
        let _ = logical_cpu;
    }
}

// ── Spot-check ──────────────────────────────────────────────────────────────

/// One spot-check outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotCheckItem {
    /// Workload name from profile.
    pub workload: String,
    /// Profile winner.
    pub profile_winner: String,
    /// Profile confidence.
    pub profile_confidence: String,
    /// Fresh measurement note.
    pub observation: String,
    /// Whether the spot-check agrees with keeping the profile entry.
    pub ok: bool,
}

/// Spot-check report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotCheckReport {
    /// Profile path label.
    pub profile_label: String,
    /// Host brand.
    pub host_brand: String,
    /// Items checked.
    pub items: Vec<SpotCheckItem>,
    /// Count OK.
    pub ok_count: usize,
    /// Count failed / inconclusive recheck.
    pub fail_count: usize,
    /// Summary.
    pub summary: String,
}

/// Spot-check up to `limit` non-inconclusive workloads with tiny samples.
pub fn spot_check_profile(
    info: &HardwareInfo,
    profile: &HnepProfile,
    limit: usize,
    cfg: &MeasurementConfig,
) -> Result<SpotCheckReport> {
    let eng = MeasurementEngine::new(cfg.clone());
    let mut items = Vec::new();
    for w in profile
        .workloads
        .iter()
        .filter(|w| w.confidence != Confidence::Inconclusive)
        .take(limit)
    {
        items.push(check_one(info, &eng, &w.name, &w.winner, w.confidence)?);
    }
    if let Some(sc) = profile.size_classes.first() {
        if items.len() < limit {
            let name = format!("memscan-{}", sc.class.to_ascii_lowercase());
            items.push(check_one(
                info,
                &eng,
                &name,
                &sc.winner,
                sc.confidence,
            )?);
        }
    }
    let ok_count = items.iter().filter(|i| i.ok).count();
    let fail_count = items.len() - ok_count;
    let summary = if items.is_empty() {
        "No non-inconclusive workloads to spot-check.".into()
    } else if fail_count == 0 {
        format!("All {ok_count} spot-checks consistent under tiny sample.")
    } else {
        format!(
            "{fail_count}/{} spot-checks unstable or disagreed — consider partial retrain.",
            items.len()
        )
    };
    Ok(SpotCheckReport {
        profile_label: profile.header.label.clone(),
        host_brand: info.brand.clone(),
        items,
        ok_count,
        fail_count,
        summary,
    })
}

fn check_one(
    info: &HardwareInfo,
    eng: &MeasurementEngine,
    name: &str,
    winner: &str,
    confidence: Confidence,
) -> Result<SpotCheckItem> {
    let (observation, ok) = if name.starts_with("memscan") || name.starts_with("memory") {
        let mut op = MemOpBench::for_target(&info.topology, CacheTarget::L2);
        let base = eng.measure(|| {
            let _ = op.run_scan_stride();
        })?;
        let cand = eng.measure(|| {
            let _ = op.run_copy();
        })?;
        let stable =
            base.stability.label() != "UNSTABLE" && cand.stability.label() != "UNSTABLE";
        (
            format!(
                "fresh L2 scan={:.0}ns copy={:.0}ns [{}|{}]",
                base.median_ns,
                cand.median_ns,
                base.stability.label(),
                cand.stability.label()
            ),
            stable,
        )
    } else if name.starts_with("float") {
        let b = FloatBench::new(2048);
        let s = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        (
            format!(
                "float baseline median={:.0}ns {}",
                s.median_ns,
                s.stability.label()
            ),
            s.stability.label() != "UNSTABLE",
        )
    } else if name.starts_with("concurrency") {
        let b = ConcurrencyBench {
            iters: 5_000,
            threads: 4.min(info.topology.thread_count().max(1)),
        };
        let base = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        let cand = eng.measure(|| {
            let _ = b.run_candidate();
        })?;
        let stable =
            base.stability.label() != "UNSTABLE" && cand.stability.label() != "UNSTABLE";
        (
            format!(
                "concurrency base={:.0} cand={:.0} profile_winner={winner}",
                base.median_ns, cand.median_ns
            ),
            stable,
        )
    } else {
        let b = IntegerBench { n: 42 };
        let s = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        (
            format!(
                "integer median={:.0}ns {} (profile_winner={winner})",
                s.median_ns,
                s.stability.label()
            ),
            s.stability.label() != "UNSTABLE",
        )
    };
    Ok(SpotCheckItem {
        workload: name.into(),
        profile_winner: winner.into(),
        profile_confidence: confidence.label().into(),
        observation,
        ok,
    })
}
