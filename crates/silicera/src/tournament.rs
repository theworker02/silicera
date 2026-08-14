//! Tournament pipeline: measure variants, gate on correctness, reject regressions.
//!
//! Ranking uses median latency. Wins require statistical confidence; otherwise
//! confidence is lowered rather than inventing a champion.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SiliceraError};
use crate::hnep::Confidence;
use crate::measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary, Stability};
use crate::variant::{Variant, VariantId, VariantRecord};

/// Tournament configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentConfig {
    /// Measurement parameters.
    pub measurement: MeasurementConfig,
    /// Minimum relative improvement over baseline to claim a win (e.g. 0.05 = 5%).
    pub min_improvement: f64,
    /// If true, unstable measurements abort the tournament.
    pub abort_on_unstable: bool,
    /// Baseline variant id (regression reference).
    pub baseline_id: VariantId,
}

impl Default for TournamentConfig {
    fn default() -> Self {
        Self {
            measurement: MeasurementConfig::default(),
            min_improvement: 0.03,
            abort_on_unstable: false,
            baseline_id: VariantId::new("baseline"),
        }
    }
}

/// Outcome for a single measured variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundOutcome {
    /// Record.
    pub record: VariantRecord,
}

/// Full tournament result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentResult {
    /// Workload name.
    pub name: String,
    /// Per-variant records.
    pub records: Vec<VariantRecord>,
    /// Selected winner id (baseline if inconclusive).
    pub winner: VariantId,
    /// Confidence in the selection.
    pub confidence: Confidence,
    /// Human-readable explanation.
    pub rationale: String,
}

/// Runs a set of variants through measurement + gates.
pub struct Tournament {
    /// Config.
    pub config: TournamentConfig,
}

impl Tournament {
    /// Create tournament.
    pub fn new(config: TournamentConfig) -> Self {
        Self { config }
    }

    /// Run tournament over homogeneous variants.
    pub fn run<V: Variant>(&self, name: &str, variants: &[V]) -> Result<TournamentResult>
    where
        V::Output: PartialEq,
    {
        if variants.is_empty() {
            return Err(SiliceraError::Internal("tournament has no variants".into()));
        }

        let engine = MeasurementEngine::new(self.config.measurement.clone());
        let mut records = Vec::new();

        // Establish reference output from baseline if present, else first variant.
        let baseline = variants
            .iter()
            .find(|v| v.id() == &self.config.baseline_id)
            .unwrap_or(&variants[0]);
        let reference = baseline.run();

        for v in variants {
            let (summary, output) = engine.measure_with_output(|| v.run())?;
            let correct = output == reference;
            let mut notes = Vec::new();
            if !correct {
                notes.push("correctness mismatch vs baseline reference".into());
            }
            match summary.stability {
                Stability::Unstable => {
                    notes.push("measurement UNSTABLE".into());
                    if self.config.abort_on_unstable {
                        return Err(SiliceraError::MeasurementUnstable(format!(
                            "variant {} unstable",
                            v.id()
                        )));
                    }
                }
                Stability::Noisy => notes.push("measurement NOISY".into()),
                Stability::Stable => {}
            }
            records.push(VariantRecord {
                id: v.id().clone(),
                summary,
                correct,
                regression: false, // filled after baseline known
                notes,
            });
        }

        let baseline_median = records
            .iter()
            .find(|r| r.id == self.config.baseline_id)
            .or_else(|| records.first())
            .map(|r| r.summary.median_ns)
            .unwrap_or(0.0);

        for r in &mut records {
            if r.correct && r.summary.median_ns > baseline_median * (1.0 + self.config.min_improvement)
            {
                // Slower than baseline by more than threshold → regression.
                // (Only mark non-baseline.)
                if r.id != self.config.baseline_id {
                    r.regression = true;
                    r.notes.push("regression vs baseline".into());
                }
            }
        }

        select_winner(name, records, &self.config, baseline_median)
    }
}

fn select_winner(
    name: &str,
    records: Vec<VariantRecord>,
    cfg: &TournamentConfig,
    baseline_median: f64,
) -> Result<TournamentResult> {
    let eligible: Vec<&VariantRecord> = records
        .iter()
        .filter(|r| r.correct && !r.regression && r.summary.stability != Stability::Unstable)
        .collect();

    if eligible.is_empty() {
        let winner = cfg.baseline_id.clone();
        return Ok(TournamentResult {
            name: name.into(),
            records,
            winner,
            confidence: Confidence::Inconclusive,
            rationale: "no eligible variants after correctness/stability gates; keeping baseline"
                .into(),
        });
    }

    let best = eligible
        .iter()
        .min_by(|a, b| {
            a.summary
                .median_ns
                .partial_cmp(&b.summary.median_ns)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap();

    let improvement = if baseline_median > 0.0 {
        (baseline_median - best.summary.median_ns) / baseline_median
    } else {
        0.0
    };

    let (confidence, rationale) = classify_confidence(best, improvement, cfg.min_improvement);

    Ok(TournamentResult {
        name: name.into(),
        winner: best.id.clone(),
        confidence,
        rationale,
        records,
    })
}

fn classify_confidence(
    best: &VariantRecord,
    improvement: f64,
    min_improvement: f64,
) -> (Confidence, String) {
    if best.summary.stability == Stability::Noisy {
        return (
            Confidence::Low,
            format!(
                "best variant {} but measurement NOISY; improvement={:.2}%",
                best.id,
                improvement * 100.0
            ),
        );
    }
    if improvement < min_improvement {
        return (
            Confidence::Inconclusive,
            format!(
                "best variant {} improvement={:.2}% below threshold {:.2}%; prefer baseline",
                best.id,
                improvement * 100.0,
                min_improvement * 100.0
            ),
        );
    }
    // Rough effect-size vs noise: improvement should exceed ~2× CV.
    let cv = if best.summary.mean_ns > 0.0 {
        best.summary.stddev_ns / best.summary.mean_ns
    } else {
        0.0
    };
    if improvement > cv * 3.0 && improvement > min_improvement * 2.0 {
        (
            Confidence::High,
            format!(
                "variant {} improves median by {:.2}% with STABLE samples (cv={:.3})",
                best.id,
                improvement * 100.0,
                cv
            ),
        )
    } else if improvement > cv * 1.5 {
        (
            Confidence::Medium,
            format!(
                "variant {} improves median by {:.2}% (cv={:.3})",
                best.id,
                improvement * 100.0,
                cv
            ),
        )
    } else {
        (
            Confidence::Low,
            format!(
                "variant {} nominal improvement {:.2}% is comparable to noise (cv={:.3})",
                best.id,
                improvement * 100.0,
                cv
            ),
        )
    }
}

/// Convenience: compare two summaries without running closures.
pub fn relative_improvement(baseline: &MeasurementSummary, candidate: &MeasurementSummary) -> f64 {
    if baseline.median_ns <= 0.0 {
        return 0.0;
    }
    (baseline.median_ns - candidate.median_ns) / baseline.median_ns
}
