//! Measurement engine with statistical summaries and instability flags.
//!
//! Correctness and statistical skepticism take priority over chasing speedups.
//! Results include median, mean, variance, percentiles, outlier counts, and
//! explicit stability classification.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{Result, SiliceraError};

/// Configuration for a measurement campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementConfig {
    /// Warmup iterations (discarded).
    pub warmup: usize,
    /// Timed iterations.
    pub iterations: usize,
    /// Maximum relative IQR / median before marking unstable (e.g. 0.25 = 25%).
    pub instability_iqr_ratio: f64,
    /// Maximum coefficient of variation (stddev/mean) before unstable.
    pub instability_cv: f64,
    /// Outlier fence in IQR multiples (Tukey; typically 1.5).
    pub outlier_fence: f64,
}

impl Default for MeasurementConfig {
    fn default() -> Self {
        Self {
            warmup: 5,
            iterations: 30,
            instability_iqr_ratio: 0.35,
            instability_cv: 0.20,
            outlier_fence: 1.5,
        }
    }
}

/// Stability classification for a sample set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stability {
    /// Tight distribution; safe for ranking.
    Stable,
    /// Elevated variance; report but do not claim strong wins.
    Noisy,
    /// Unusable for specialization decisions.
    Unstable,
}

impl Stability {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            Stability::Stable => "STABLE",
            Stability::Noisy => "NOISY",
            Stability::Unstable => "UNSTABLE",
        }
    }
}

/// Statistical summary of timed samples (nanoseconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementSummary {
    /// Sample count (after warmup).
    pub n: usize,
    /// Minimum (ns).
    pub min_ns: f64,
    /// Maximum (ns).
    pub max_ns: f64,
    /// Arithmetic mean (ns).
    pub mean_ns: f64,
    /// Median (ns).
    pub median_ns: f64,
    /// Sample variance (ns²).
    pub variance_ns2: f64,
    /// Sample standard deviation (ns).
    pub stddev_ns: f64,
    /// 25th percentile (ns).
    pub p25_ns: f64,
    /// 75th percentile (ns).
    pub p75_ns: f64,
    /// 95th percentile (ns).
    pub p95_ns: f64,
    /// 99th percentile (ns).
    pub p99_ns: f64,
    /// Tukey outlier count.
    pub outlier_count: usize,
    /// Stability classification.
    pub stability: Stability,
    /// Human-readable flags (e.g. high CV).
    pub flags: Vec<String>,
}

impl MeasurementSummary {
    /// Compute summary from raw nanosecond samples.
    pub fn from_samples(samples: &[f64], cfg: &MeasurementConfig) -> Result<Self> {
        if samples.is_empty() {
            return Err(SiliceraError::MeasurementUnstable(
                "no samples collected".into(),
            ));
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let min_ns = sorted[0];
        let max_ns = sorted[n - 1];
        let mean_ns = sorted.iter().sum::<f64>() / n as f64;
        let median_ns = percentile_sorted(&sorted, 0.50);
        let p25_ns = percentile_sorted(&sorted, 0.25);
        let p75_ns = percentile_sorted(&sorted, 0.75);
        let p95_ns = percentile_sorted(&sorted, 0.95);
        let p99_ns = percentile_sorted(&sorted, 0.99);
        let variance_ns2 = if n > 1 {
            sorted.iter().map(|x| {
                let d = x - mean_ns;
                d * d
            }).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        let stddev_ns = variance_ns2.sqrt();
        let iqr = p75_ns - p25_ns;
        let lo = p25_ns - cfg.outlier_fence * iqr;
        let hi = p75_ns + cfg.outlier_fence * iqr;
        let outlier_count = sorted.iter().filter(|&&x| x < lo || x > hi).count();

        let mut flags = Vec::new();
        let cv = if mean_ns > 0.0 {
            stddev_ns / mean_ns
        } else {
            0.0
        };
        let iqr_ratio = if median_ns > 0.0 {
            iqr / median_ns
        } else {
            0.0
        };

        if cv > cfg.instability_cv {
            flags.push(format!("cv={cv:.3} > {}", cfg.instability_cv));
        }
        if iqr_ratio > cfg.instability_iqr_ratio {
            flags.push(format!(
                "iqr/median={iqr_ratio:.3} > {}",
                cfg.instability_iqr_ratio
            ));
        }
        if outlier_count * 5 > n {
            flags.push(format!("outliers={outlier_count}/{n}"));
        }

        let stability = if cv > cfg.instability_cv * 1.5
            || iqr_ratio > cfg.instability_iqr_ratio * 1.5
        {
            Stability::Unstable
        } else if !flags.is_empty() {
            Stability::Noisy
        } else {
            Stability::Stable
        };

        Ok(Self {
            n,
            min_ns,
            max_ns,
            mean_ns,
            median_ns,
            variance_ns2,
            stddev_ns,
            p25_ns,
            p75_ns,
            p95_ns,
            p99_ns,
            outlier_count,
            stability,
            flags,
        })
    }

    /// Primary ranking key (median nanoseconds; lower is better).
    pub fn score_ns(&self) -> f64 {
        self.median_ns
    }
}

/// Runs timed closures with warmup and statistical aggregation.
#[derive(Debug, Clone)]
pub struct MeasurementEngine {
    /// Configuration.
    pub config: MeasurementConfig,
}

impl Default for MeasurementEngine {
    fn default() -> Self {
        Self {
            config: MeasurementConfig::default(),
        }
    }
}

impl MeasurementEngine {
    /// Create with config.
    pub fn new(config: MeasurementConfig) -> Self {
        Self { config }
    }

    /// Measure a closure that returns a unit value (timing only).
    pub fn measure<F>(&self, mut f: F) -> Result<MeasurementSummary>
    where
        F: FnMut(),
    {
        for _ in 0..self.config.warmup {
            f();
            // Prevent the optimizer from considering the loop dead in trivial cases.
            std::hint::black_box(());
        }
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            f();
            let elapsed = start.elapsed();
            samples.push(duration_ns(elapsed));
            std::hint::black_box(());
        }
        MeasurementSummary::from_samples(&samples, &self.config)
    }

    /// Measure a closure that also produces an output for correctness checking.
    pub fn measure_with_output<F, T>(&self, mut f: F) -> Result<(MeasurementSummary, T)>
    where
        F: FnMut() -> T,
        T: Clone,
    {
        let mut last = None;
        for _ in 0..self.config.warmup {
            last = Some(f());
            std::hint::black_box(());
        }
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let out = f();
            let elapsed = start.elapsed();
            samples.push(duration_ns(elapsed));
            last = Some(out);
            std::hint::black_box(());
        }
        let summary = MeasurementSummary::from_samples(&samples, &self.config)?;
        let out = last.ok_or_else(|| {
            SiliceraError::Internal("measure_with_output produced no output".into())
        })?;
        Ok((summary, out))
    }
}

fn duration_ns(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000_000_000.0
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = p * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let w = idx - lo as f64;
        sorted[lo] * (1.0 - w) + sorted[hi] * w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_stable_constant() {
        let samples: Vec<f64> = (0..40).map(|_| 1000.0).collect();
        let s = MeasurementSummary::from_samples(&samples, &MeasurementConfig::default()).unwrap();
        assert_eq!(s.stability, Stability::Stable);
        assert_eq!(s.median_ns, 1000.0);
    }

    #[test]
    fn engine_runs() {
        let eng = MeasurementEngine::new(MeasurementConfig {
            warmup: 1,
            iterations: 5,
            ..Default::default()
        });
        let mut x = 0u64;
        let s = eng
            .measure(|| {
                x = x.wrapping_add(1);
                std::hint::black_box(x);
            })
            .unwrap();
        assert_eq!(s.n, 5);
    }
}
