//! Optional hardware-counter backend.
//!
//! Specialization decisions use wall-clock timers. Hardware performance
//! counters are an **optional evidence channel** and must never be required.
//!
//! # Windows (unprivileged)
//!
//! Reading PMCs typically needs elevated privileges or a kernel driver
//! (`perf` on Linux; ETW/PMC APIs on Windows). Without privileges this module
//! reports `Unavailable` and callers continue with timers only.
//!
//! Do not invent counter values. Document the limitation when publishing.

use serde::{Deserialize, Serialize};

/// Backend availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CounterAvailability {
    /// Counters not compiled / not attempted.
    Disabled,
    /// Attempted; OS denied or no unprivileged path.
    Unavailable,
    /// Backend present (still may fail per-event).
    Available,
}

impl CounterAvailability {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            CounterAvailability::Disabled => "DISABLED",
            CounterAvailability::Unavailable => "UNAVAILABLE",
            CounterAvailability::Available => "AVAILABLE",
        }
    }
}

/// Probe result for optional counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterProbe {
    /// Availability.
    pub availability: CounterAvailability,
    /// Platform note.
    pub platform: String,
    /// Human-readable limitation.
    pub limitation: String,
    /// Suggested evidence channel when counters are unavailable.
    pub fallback: String,
}

/// Probe whether an unprivileged HW-counter path exists on this host.
///
/// Always safe: never raises privileges; never panics.
pub fn probe_counters() -> CounterProbe {
    #[cfg(target_os = "windows")]
    {
        CounterProbe {
            availability: CounterAvailability::Unavailable,
            platform: "windows".into(),
            limitation: "Unprivileged PMC access is not available on Windows \
                 (would require elevated ETW/PMC or a driver). Silicera does not request \
                 privileges."
                .into(),
            fallback: "wall-clock MeasurementEngine (median / stability flags)".into(),
        }
    }
    #[cfg(target_os = "linux")]
    {
        // We do not open perf_event_open here — that can fail without CAP_PERFMON
        // and is out of current release scope. Report unavailable unless a future feature
        // wires a real backend.
        CounterProbe {
            availability: CounterAvailability::Unavailable,
            platform: "linux".into(),
            limitation: "This release does not open perf_event_open; enable a future `counters` \
                 feature once a least-privilege path is validated. Specialization works \
                 without PMCs."
                .into(),
            fallback: "wall-clock MeasurementEngine (median / stability flags)".into(),
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        CounterProbe {
            availability: CounterAvailability::Disabled,
            platform: std::env::consts::OS.into(),
            limitation: "No hardware-counter backend for this OS in this release.".into(),
            fallback: "wall-clock MeasurementEngine (median / stability flags)".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_never_panics() {
        let p = probe_counters();
        assert!(!p.fallback.is_empty());
        assert!(matches!(
            p.availability,
            CounterAvailability::Disabled
                | CounterAvailability::Unavailable
                | CounterAvailability::Available
        ));
    }
}
