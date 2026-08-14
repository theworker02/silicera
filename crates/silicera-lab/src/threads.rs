//! Thread-count / core-placement specialization experiments.
//!
//! Silicera-owned only: we vary the thread count passed into lab workloads and
//! measure. We do **not** change global OS scheduling, affinity, or power plans.
//! More threads is not assumed faster.

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary};
use silicera::Result;

use crate::bench::ConcurrencyBench;

/// One measured thread-count point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadCountPoint {
    /// Thread count used for this measurement.
    pub threads: usize,
    /// Contended-atomic baseline median.
    pub baseline: MeasurementSummary,
    /// Per-thread-local candidate median.
    pub candidate: MeasurementSummary,
    /// Which variant had lower median (`baseline` or `candidate`).
    pub faster: String,
}

/// Report for the thread-count sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadCountReport {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Logical CPUs reported by topology / OS.
    pub logical_cpus: usize,
    /// Iterations across all threads (divided among threads; constant total work).
    pub iters_total: u64,
    /// Measured points.
    pub points: Vec<ThreadCountPoint>,
    /// Thread count with lowest candidate median (among measured).
    pub best_candidate_threads: usize,
    /// Thread count with lowest baseline median.
    pub best_baseline_threads: usize,
    /// Honest conclusion.
    pub conclusion: String,
    /// Methodology note.
    pub note: String,
}

/// Default thread counts to sweep (clamped to host logical CPUs).
pub fn default_thread_sweep(logical_cpus: usize) -> Vec<usize> {
    let max = logical_cpus.max(1);
    let mut v = vec![1usize, 2, 4, 8, 16];
    v.retain(|&t| t <= max);
    if !v.contains(&max) && max > 1 {
        v.push(max);
    }
    if v.is_empty() {
        v.push(1);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Measure concurrency kernels across thread counts (Silicera-owned; no OS affinity).
///
/// Total work is held approximately constant: `iters_per_thread * threads ≈
/// iters_per_thread_at_1` so wall-time comparisons ask whether parallelism helps,
/// not whether doing N× more work is slower.
pub fn measure_thread_count_sweep(
    info: &HardwareInfo,
    mcfg: &MeasurementConfig,
    total_iters: u64,
    thread_counts: Option<&[usize]>,
) -> Result<ThreadCountReport> {
    let logical = info
        .topology
        .thread_count()
        .max(info.environment.logical_cpus)
        .max(1);
    let counts: Vec<usize> = thread_counts
        .map(|s| s.to_vec())
        .unwrap_or_else(|| default_thread_sweep(logical));
    let eng = MeasurementEngine::new(mcfg.clone());
    let mut points = Vec::new();

    for &threads in &counts {
        let iters = (total_iters / threads as u64).max(1);
        let bench = ConcurrencyBench {
            iters,
            threads,
        };
        let baseline = eng.measure(|| {
            let _ = bench.run_baseline();
        })?;
        let candidate = eng.measure(|| {
            let _ = bench.run_candidate();
        })?;
        let faster = if candidate.median_ns < baseline.median_ns {
            "candidate"
        } else {
            "baseline"
        };
        points.push(ThreadCountPoint {
            threads,
            baseline,
            candidate,
            faster: faster.into(),
        });
    }

    let best_candidate_threads = points
        .iter()
        .min_by(|a, b| {
            a.candidate
                .median_ns
                .partial_cmp(&b.candidate.median_ns)
                .unwrap_or(std::cmp::Ordering::Less)
        })
        .map(|p| p.threads)
        .unwrap_or(1);
    let best_baseline_threads = points
        .iter()
        .min_by(|a, b| {
            a.baseline
                .median_ns
                .partial_cmp(&b.baseline.median_ns)
                .unwrap_or(std::cmp::Ordering::Less)
        })
        .map(|p| p.threads)
        .unwrap_or(1);

    let more_not_always = points.len() >= 2
        && points.last().map(|p| p.candidate.median_ns).unwrap_or(0.0)
            > points
                .iter()
                .map(|p| p.candidate.median_ns)
                .fold(f64::INFINITY, f64::min)
                + f64::EPSILON;

    let conclusion = if more_not_always {
        format!(
            "More threads is NOT always faster on this host. Best candidate median at \
             threads={best_candidate_threads}; best baseline at threads={best_baseline_threads}."
        )
    } else {
        format!(
            "Among measured counts, best candidate median at threads={best_candidate_threads}; \
             best baseline at threads={best_baseline_threads}. Do not extrapolate beyond measured points."
        )
    };

    Ok(ThreadCountReport {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        logical_cpus: logical,
        iters_total: total_iters,
        points,
        best_candidate_threads,
        best_baseline_threads,
        conclusion,
        note: "Silicera varies only the thread count argument to ConcurrencyBench; \
               total iteration budget is split across threads (constant work). \
               No SetThreadAffinityMask / sched_setaffinity / OS scheduler changes."
            .into(),
    })
}
