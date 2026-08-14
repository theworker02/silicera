//! Profile generation from lab tournaments — including size-class memscan/copy
//! and size-scaled float/integer targets.

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::hnep::{HnepProfile, SizeClassEntry};
use silicera::measure::{MeasurementConfig, MeasurementSummary};
use silicera::retrain::{parse_only_filter, target_matches_filter};
use silicera::specialize::{DecisionTree, SpecializeConfig};
use silicera::tournament::{Tournament, TournamentConfig, TournamentResult};
use silicera::variant::{Variant, VariantId};
use silicera::Result;

use crate::bench::{
    BranchBench, CacheTarget, ConcurrencyBench, FloatBench, IntegerBench, MemOpBench, MemoryBench,
};

/// Configuration for profile generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileGenConfig {
    /// Measurement config.
    pub measurement: MeasurementConfig,
    /// Output label.
    pub label: String,
    /// Include size-dependent decision tree compiled from measured size classes.
    pub size_tree: bool,
    /// Run L1/L2/L3/DRAM memscan tournaments into `size_classes`.
    pub size_class_tournaments: bool,
    /// Also measure float/integer at size-class working sets (extra evidence).
    pub extended_size_targets: bool,
    /// Minimum relative improvement for tournament wins.
    pub min_improvement: f64,
    /// Optional comma-separated `--only` filter (partial retrain).
    pub only: Option<String>,
}

impl Default for ProfileGenConfig {
    fn default() -> Self {
        Self {
            measurement: MeasurementConfig {
                warmup: 3,
                iterations: 20,
                ..Default::default()
            },
            label: "lab-train".into(),
            size_tree: true,
            size_class_tournaments: true,
            extended_size_targets: true,
            min_improvement: 0.03,
            only: None,
        }
    }
}

struct IdVariant {
    id: VariantId,
    desc: &'static str,
    body: Box<dyn Fn() + Send + Sync>,
}

impl Variant for IdVariant {
    type Output = u64;
    fn id(&self) -> &VariantId {
        &self.id
    }
    fn description(&self) -> &str {
        self.desc
    }
    fn run(&self) -> Self::Output {
        (self.body)();
        1
    }
}

/// Generate an HNEP by running lab tournaments on `info`.
pub fn generate_profile(
    info: &HardwareInfo,
    cfg: &ProfileGenConfig,
    out: &Path,
) -> Result<(HnepProfile, Vec<TournamentResult>)> {
    let fp = info.fingerprint.as_ref().ok_or_else(|| {
        silicera::SiliceraError::UnsupportedCpu(
            "cannot train: host is not a supported AMD Zen machine".into(),
        )
    })?;

    let filter = cfg
        .only
        .as_deref()
        .map(parse_only_filter)
        .unwrap_or_default();

    let mut results = Vec::new();
    if target_matches_filter("memory-l2", &filter) {
        results.push(tournament_memory(info, &cfg.measurement, cfg.min_improvement)?);
    }
    if target_matches_filter("integer", &filter) {
        results.push(tournament_integer(&cfg.measurement, cfg.min_improvement)?);
    }
    if target_matches_filter("float", &filter) {
        results.push(tournament_float(&cfg.measurement, cfg.min_improvement)?);
    }
    if target_matches_filter("branch", &filter) {
        results.push(tournament_branch(&cfg.measurement, cfg.min_improvement)?);
    }
    if target_matches_filter("concurrency", &filter) {
        results.push(tournament_concurrency(
            info,
            &cfg.measurement,
            cfg.min_improvement,
        )?);
    }

    let size_classes = if cfg.size_class_tournaments
        && (filter.is_empty() || target_matches_filter("memscan-l1", &filter) || filter.iter().any(|f| f == "size_classes" || f.starts_with("memscan")))
    {
        let (entries, sc_results) =
            measure_size_classes(info, &cfg.measurement, cfg.min_improvement, &filter)?;
        results.extend(sc_results);
        entries
    } else {
        Vec::new()
    };

    if cfg.extended_size_targets {
        let (ext_entries, ext_results) =
            measure_extended_size_targets(info, &cfg.measurement, cfg.min_improvement, &filter)?;
        // Extended targets go into workloads (not decision-tree size_classes) so
        // memscan remains the tree source of truth.
        let _ = ext_entries;
        results.extend(ext_results);
    }

    let tree = if cfg.size_tree && (filter.is_empty() || size_classes.len() == 4 || filter.iter().any(|f| f == "size_classes")) {
        Some(if size_classes.is_empty() {
            DecisionTree::from_topology(
                &info.topology,
                "l1_resident",
                "l2_resident",
                "l3_resident",
                "dram_friendly",
                "baseline",
            )
        } else {
            DecisionTree::from_size_classes(&info.topology, &size_classes, "baseline")
        })
    } else {
        let _ = SpecializeConfig::default();
        None
    };

    // Partial retrain: merge with existing profile when --only is set and file exists.
    let profile = if !filter.is_empty() && out.exists() {
        merge_partial_profile(info, fp, cfg, out, results.clone(), size_classes, tree)?
    } else {
        HnepProfile::from_tournaments(
            fp,
            info.environment.clone(),
            &results,
            size_classes,
            tree,
            cfg.label.clone(),
        )?
    };
    profile.write_to(out)?;
    Ok((profile, results))
}

fn merge_partial_profile(
    info: &HardwareInfo,
    fp: &silicera::Fingerprint,
    cfg: &ProfileGenConfig,
    out: &Path,
    new_results: Vec<TournamentResult>,
    new_size_classes: Vec<SizeClassEntry>,
    new_tree: Option<DecisionTree>,
) -> Result<HnepProfile> {
    let mut existing = HnepProfile::read_from(out)?;
    // Replace workloads that were re-measured.
    for r in &new_results {
        let winner_rec = r.records.iter().find(|x| x.id == r.winner);
        let baseline_rec = r.records.iter().find(|x| x.id.0 == "baseline");
        let entry = silicera::WorkloadEntry {
            name: r.name.clone(),
            winner: r.winner.0.clone(),
            confidence: r.confidence,
            rationale: r.rationale.clone(),
            winner_median_ns: winner_rec.map(|w| w.summary.median_ns),
            baseline_median_ns: baseline_rec.map(|b| b.summary.median_ns),
        };
        if let Some(slot) = existing.workloads.iter_mut().find(|w| w.name == r.name) {
            *slot = entry;
        } else {
            existing.workloads.push(entry);
        }
    }
    for sc in new_size_classes {
        if let Some(slot) = existing
            .size_classes
            .iter_mut()
            .find(|c| c.class == sc.class)
        {
            *slot = sc;
        } else {
            existing.size_classes.push(sc);
        }
    }
    if let Some(tree) = new_tree {
        existing.decision_tree = Some(tree);
    } else if !existing.size_classes.is_empty() {
        existing.decision_tree = Some(DecisionTree::from_size_classes(
            &info.topology,
            &existing.size_classes,
            "baseline",
        ));
    }
    existing.environment = info.environment.clone();
    existing.header.fingerprint = fp.value.clone();
    existing.header.label = cfg.label.clone();
    existing.header.created_at = chrono::Utc::now().to_rfc3339();
    existing.header.silicera_version = silicera::VERSION.into();
    existing.header.version = silicera::HNEP_VERSION;
    existing.recompute_digest()?;
    Ok(existing)
}

/// Document when size-class winners do/don't change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeClassWinnerStory {
    /// Per-class winner id.
    pub winners: Vec<(String, String)>,
    /// True when not all winners are identical.
    pub winners_differ: bool,
    /// Human summary.
    pub summary: String,
}

/// Summarize measured size-class winners.
pub fn size_class_winner_story(classes: &[SizeClassEntry]) -> SizeClassWinnerStory {
    let winners: Vec<(String, String)> = classes
        .iter()
        .map(|c| (c.class.clone(), c.winner.clone()))
        .collect();
    let winners_differ = winners
        .first()
        .map(|(_, w)| winners.iter().any(|(_, x)| x != w))
        .unwrap_or(false);
    let summary = if winners.is_empty() {
        "No size-class measurements".into()
    } else if winners_differ {
        format!(
            "Winners DO change by size: {}",
            winners
                .iter()
                .map(|(c, w)| format!("{c}={w}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "Winners do NOT change by size on this host/run (all → {})",
            winners[0].1
        )
    };
    SizeClassWinnerStory {
        winners,
        winners_differ,
        summary,
    }
}

/// Measure memscan/copy variants across L1/L2/L3/DRAM; return HNEP size-class entries.
pub fn measure_size_classes(
    info: &HardwareInfo,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
    filter: &[String],
) -> Result<(Vec<SizeClassEntry>, Vec<TournamentResult>)> {
    let mut entries = Vec::new();
    let mut results = Vec::new();
    for target in CacheTarget::all() {
        let name = format!("memscan-{}", target.class_id().to_lowercase());
        if !target_matches_filter(&name, filter) {
            continue;
        }
        let (entry, result) = tournament_memop_size_class(info, target, mcfg, min_improvement)?;
        entries.push(entry);
        results.push(result);
    }
    Ok((entries, results))
}

/// Float / integer tournaments at each cache working-set size (evidence, not tree).
fn measure_extended_size_targets(
    info: &HardwareInfo,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
    filter: &[String],
) -> Result<(Vec<SizeClassEntry>, Vec<TournamentResult>)> {
    let mut results = Vec::new();
    for target in CacheTarget::all() {
        let ws = target.size_bytes(&info.topology) as usize;
        let fname = format!("float-{}", target.class_id().to_lowercase());
        if target_matches_filter(&fname, filter) {
            let n = (ws / 8).clamp(256, 1 << 20);
            results.push(run_pair(
                &fname,
                mcfg,
                min_improvement,
                {
                    let a = FloatBench::new(n);
                    move || {
                        let _ = a.run_baseline();
                    }
                },
                {
                    let b = FloatBench::new(n);
                    move || {
                        let _ = b.run_candidate();
                    }
                },
            )?);
        }
        let iname = format!("intmix-{}", target.class_id().to_lowercase());
        if target_matches_filter(&iname, filter) {
            // Integer mix is iteration-bound; scale loop via seed only (fixed work).
            results.push(tournament_integer_named(&iname, mcfg, min_improvement)?);
        }
    }
    Ok((Vec::new(), results))
}

fn tournament_memop_size_class(
    info: &HardwareInfo,
    target: CacheTarget,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
) -> Result<(SizeClassEntry, TournamentResult)> {
    let name = format!("memscan-{}", target.class_id().to_lowercase());
    let bench = Arc::new(Mutex::new(MemOpBench::for_target(&info.topology, target)));
    let b_scan = Arc::clone(&bench);
    let b_dense = Arc::clone(&bench);
    let b_copy = Arc::clone(&bench);
    let b_loop = Arc::clone(&bench);
    let b_unroll = Arc::clone(&bench);

    let variants = vec![
        IdVariant {
            id: VariantId::new("baseline"),
            desc: "scan_stride",
            body: Box::new(move || {
                let g = b_scan.lock().unwrap();
                let _ = g.run_scan_stride();
            }),
        },
        IdVariant {
            id: VariantId::new("scan_dense"),
            desc: "scan_dense",
            body: Box::new(move || {
                let g = b_dense.lock().unwrap();
                let _ = g.run_scan_dense();
            }),
        },
        IdVariant {
            id: VariantId::new("copy"),
            desc: "memcpy",
            body: Box::new(move || {
                let mut g = b_copy.lock().unwrap();
                let _ = g.run_copy();
            }),
        },
        IdVariant {
            id: VariantId::new("copy_loop"),
            desc: "copy_loop",
            body: Box::new(move || {
                let mut g = b_loop.lock().unwrap();
                let _ = g.run_copy_loop();
            }),
        },
        IdVariant {
            id: VariantId::new("copy_unrolled8"),
            desc: "copy_unrolled8",
            body: Box::new(move || {
                let mut g = b_unroll.lock().unwrap();
                let _ = g.run_copy_unrolled8();
            }),
        },
    ];

    let result = Tournament::new(TournamentConfig {
        measurement: mcfg.clone(),
        baseline_id: VariantId::new("baseline"),
        min_improvement,
        ..Default::default()
    })
    .run(&name, &variants)?;

    let winner_rec = result.records.iter().find(|x| x.id == result.winner);
    let baseline_rec = result.records.iter().find(|x| x.id.0 == "baseline");
    let entry = SizeClassEntry {
        class: target.class_id().into(),
        threshold_bytes: target.threshold_bytes(&info.topology),
        working_set_bytes: target.size_bytes(&info.topology),
        winner: result.winner.0.clone(),
        confidence: result.confidence,
        rationale: result.rationale.clone(),
        winner_median_ns: winner_rec.map(|w| w.summary.median_ns),
        baseline_median_ns: baseline_rec.map(|b| b.summary.median_ns),
    };
    Ok((entry, result))
}

fn run_pair(
    name: &str,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
    baseline: impl Fn() + Send + Sync + 'static,
    candidate: impl Fn() + Send + Sync + 'static,
) -> Result<TournamentResult> {
    let variants = vec![
        IdVariant {
            id: VariantId::new("baseline"),
            desc: "baseline",
            body: Box::new(baseline),
        },
        IdVariant {
            id: VariantId::new("candidate"),
            desc: "candidate",
            body: Box::new(candidate),
        },
    ];
    Tournament::new(TournamentConfig {
        measurement: mcfg.clone(),
        baseline_id: VariantId::new("baseline"),
        min_improvement,
        ..Default::default()
    })
    .run(name, &variants)
}

fn tournament_memory(
    info: &HardwareInfo,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
) -> Result<TournamentResult> {
    let b1 = MemoryBench::for_target(&info.topology, CacheTarget::L2);
    let b2 = MemoryBench::for_target(&info.topology, CacheTarget::L2);
    run_pair(
        "memory-l2",
        mcfg,
        min_improvement,
        move || {
            let _ = b1.run();
        },
        move || {
            let _ = b2.run_prefetch_friendly();
        },
    )
}

fn tournament_integer(mcfg: &MeasurementConfig, min_improvement: f64) -> Result<TournamentResult> {
    tournament_integer_named("integer", mcfg, min_improvement)
}

fn tournament_integer_named(
    name: &str,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
) -> Result<TournamentResult> {
    let a = IntegerBench { n: 42 };
    let b = IntegerBench { n: 42 };
    run_pair(
        name,
        mcfg,
        min_improvement,
        move || {
            let _ = a.run_baseline();
        },
        move || {
            let _ = b.run_candidate();
        },
    )
}

fn tournament_float(mcfg: &MeasurementConfig, min_improvement: f64) -> Result<TournamentResult> {
    let a = FloatBench::new(4096);
    let b = FloatBench::new(4096);
    run_pair(
        "float",
        mcfg,
        min_improvement,
        move || {
            let _ = a.run_baseline();
        },
        move || {
            let _ = b.run_candidate();
        },
    )
}

fn tournament_branch(mcfg: &MeasurementConfig, min_improvement: f64) -> Result<TournamentResult> {
    let a = BranchBench::new(8192);
    let b = BranchBench::new(8192);
    run_pair(
        "branch",
        mcfg,
        min_improvement,
        move || {
            let _ = a.run_baseline();
        },
        move || {
            let _ = b.run_candidate();
        },
    )
}

fn tournament_concurrency(
    info: &HardwareInfo,
    mcfg: &MeasurementConfig,
    min_improvement: f64,
) -> Result<TournamentResult> {
    let threads = 4.min(info.topology.thread_count().max(1));
    let a = ConcurrencyBench {
        iters: 20_000,
        threads,
    };
    let b = ConcurrencyBench {
        iters: 20_000,
        threads,
    };
    run_pair(
        "concurrency",
        mcfg,
        min_improvement,
        move || {
            let _ = a.run_baseline();
        },
        move || {
            let _ = b.run_candidate();
        },
    )
}

/// Per-size dispatch overhead sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchSizeSample {
    /// Working-set probe size (bytes).
    pub size_bytes: u64,
    /// Direct stand-in median ns (single evaluate path amortized in batch).
    pub direct_median_ns: f64,
    /// Via-tree median ns.
    pub tree_median_ns: f64,
    /// Delta (tree − direct).
    pub delta_ns: f64,
}

/// Statistical summary of decision-tree dispatch overhead across sizes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchOverheadReport {
    /// Batch measurement (all sizes in one timed closure) — legacy comparable.
    pub batch_direct: MeasurementSummary,
    /// Batch via tree.
    pub batch_tree: MeasurementSummary,
    /// Batch delta ns (tree − direct medians).
    pub batch_delta_ns: f64,
    /// Per-size samples (separate timed campaigns).
    pub per_size: Vec<DispatchSizeSample>,
    /// Median of per-size deltas.
    pub delta_median_ns: f64,
    /// Mean of per-size deltas.
    pub delta_mean_ns: f64,
    /// Min / max of per-size deltas.
    pub delta_min_ns: f64,
    /// Max delta.
    pub delta_max_ns: f64,
    /// Honest note.
    pub note: String,
}

/// Measure decision-tree dispatch overhead vs a direct function call.
///
/// Returns batch summaries (compatible with older call sites) plus a multi-size
/// statistical report via [`measure_dispatch_overhead_report`].
pub fn measure_dispatch_overhead(
    tree: &DecisionTree,
    mcfg: &MeasurementConfig,
) -> Result<(MeasurementSummary, MeasurementSummary)> {
    let report = measure_dispatch_overhead_report(tree, mcfg)?;
    Ok((report.batch_direct, report.batch_tree))
}

/// Multi-size dispatch overhead with statistical summary.
pub fn measure_dispatch_overhead_report(
    tree: &DecisionTree,
    mcfg: &MeasurementConfig,
) -> Result<DispatchOverheadReport> {
    use silicera::measure::MeasurementEngine;
    let eng = MeasurementEngine::new(mcfg.clone());
    let sizes = [1024u64, 16_384, 100_000, 1_000_000, 2_000_000, 64_000_000];
    const INNER: u64 = 4_096;

    let batch_direct = eng.measure(|| {
        let mut x = 0u64;
        for &s in &sizes {
            for i in 0..INNER {
                x = std::hint::black_box(
                    x.wrapping_add(s)
                        .wrapping_mul(3)
                        .wrapping_add(i)
                        .wrapping_add(1),
                );
            }
        }
        std::hint::black_box(x);
    })?;
    let batch_tree = eng.measure(|| {
        let mut x = 0u64;
        for &s in &sizes {
            for i in 0..INNER {
                let v = tree.evaluate(std::hint::black_box(s.wrapping_add(i % 17)));
                let mix = v.as_bytes().iter().fold(0u64, |a, &b| a.wrapping_add(b as u64));
                x = std::hint::black_box(
                    x.wrapping_add(mix)
                        .wrapping_add(s)
                        .wrapping_mul(3)
                        .wrapping_add(i)
                        .wrapping_add(1),
                );
            }
        }
        std::hint::black_box(x);
    })?;

    let mut per_size = Vec::new();
    for &s in &sizes {
        let direct = eng.measure(|| {
            let mut x = 0u64;
            for i in 0..INNER {
                x = std::hint::black_box(x.wrapping_add(s).wrapping_add(i).wrapping_mul(3));
            }
            std::hint::black_box(x);
        })?;
        let via = eng.measure(|| {
            let mut x = 0u64;
            for i in 0..INNER {
                let v = tree.evaluate(std::hint::black_box(s.wrapping_add(i % 17)));
                let mix = v.as_bytes().iter().fold(0u64, |a, &b| a.wrapping_add(b as u64));
                x = std::hint::black_box(x.wrapping_add(mix).wrapping_add(s).wrapping_add(i));
            }
            std::hint::black_box(x);
        })?;
        per_size.push(DispatchSizeSample {
            size_bytes: s,
            direct_median_ns: direct.median_ns,
            tree_median_ns: via.median_ns,
            delta_ns: via.median_ns - direct.median_ns,
        });
    }

    let mut deltas: Vec<f64> = per_size.iter().map(|p| p.delta_ns).collect();
    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));
    let delta_median_ns = if deltas.is_empty() {
        0.0
    } else if deltas.len() % 2 == 0 {
        (deltas[deltas.len() / 2 - 1] + deltas[deltas.len() / 2]) / 2.0
    } else {
        deltas[deltas.len() / 2]
    };
    let delta_mean_ns = if deltas.is_empty() {
        0.0
    } else {
        deltas.iter().sum::<f64>() / deltas.len() as f64
    };
    let delta_min_ns = deltas.first().copied().unwrap_or(0.0);
    let delta_max_ns = deltas.last().copied().unwrap_or(0.0);

    Ok(DispatchOverheadReport {
        batch_delta_ns: batch_tree.median_ns - batch_direct.median_ns,
        batch_direct,
        batch_tree,
        per_size,
        delta_median_ns,
        delta_mean_ns,
        delta_min_ns,
        delta_max_ns,
        note: "Heavy stand-in (4096 iters × sizes): tree.evaluate + string mix vs direct add. \
               Not a full variant call. Host-specific; quiet system recommended."
            .into(),
    })
}
