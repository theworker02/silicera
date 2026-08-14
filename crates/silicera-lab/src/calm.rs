//! Soft environment calm / noise probe (single machine).
//!
//! Detects unstable measurement conditions (background load, migration noise)
//! without privileged counters. Advisory only — never modifies OS power policy.

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::measure::{MeasurementConfig, MeasurementEngine, Stability};
use silicera::Result;

/// Calm-check report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalmCheckReport {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// First campaign stability.
    pub pass_a: String,
    /// Second campaign stability.
    pub pass_b: String,
    /// Median ns pass A (constant work).
    pub median_a_ns: f64,
    /// Median ns pass B.
    pub median_b_ns: f64,
    /// Relative drift |A-B|/max(A,B).
    pub relative_drift: f64,
    /// Whether the host looks calm enough for specialization.
    pub calm: bool,
    /// Recommendation.
    pub recommendation: String,
}

/// Run two back-to-back constant workloads; flag drift / instability.
pub fn run_calm_check(info: &HardwareInfo, iterations: usize) -> Result<CalmCheckReport> {
    let eng = MeasurementEngine::new(MeasurementConfig {
        warmup: 5,
        iterations: iterations.max(20),
        ..Default::default()
    });
    let a = eng.measure(|| {
        let mut x = 1u64;
        for i in 0..50_000u64 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223).wrapping_add(i);
        }
        std::hint::black_box(x);
    })?;
    // Brief yield so scheduler noise can appear between passes.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let b = eng.measure(|| {
        let mut x = 1u64;
        for i in 0..50_000u64 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223).wrapping_add(i);
        }
        std::hint::black_box(x);
    })?;
    let denom = a.median_ns.max(b.median_ns).max(1.0);
    let relative_drift = (a.median_ns - b.median_ns).abs() / denom;
    let calm = a.stability != Stability::Unstable
        && b.stability != Stability::Unstable
        && relative_drift < 0.15;
    let recommendation = if calm {
        "Host looks calm enough for specialization under this soft probe.".into()
    } else {
        "MEASUREMENT ENVIRONMENT MAY BE NOISY — close background load, re-run calm-check, \
         then retrain. Do not trust razor-thin specialization margins."
            .into()
    };
    Ok(CalmCheckReport {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        pass_a: a.stability.label().into(),
        pass_b: b.stability.label().into(),
        median_a_ns: a.median_ns,
        median_b_ns: b.median_ns,
        relative_drift,
        calm,
        recommendation,
    })
}
