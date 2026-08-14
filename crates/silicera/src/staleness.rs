//! Profile staleness detection and retrain recommendations.
//!
//! Compares a stored HNEP environment snapshot to the live host. Drift does not
//! invent winners — it only tells operators what to re-measure.

use serde::{Deserialize, Serialize};

use crate::hardware::EnvironmentSnapshot;
use crate::hnep::HnepProfile;

/// How severe an environment drift is for specialization trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DriftSeverity {
    /// No actionable drift.
    None,
    /// Soft drift — profile still usable; refresh recommended.
    Soft,
    /// Hard drift — treat as untrusted for specialized dispatch.
    Hard,
}

impl DriftSeverity {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            DriftSeverity::None => "NONE",
            DriftSeverity::Soft => "SOFT",
            DriftSeverity::Hard => "HARD",
        }
    }
}

/// One detected drift signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftSignal {
    /// Field that drifted (`os`, `arch`, `logical_cpus`, `age_days`, …).
    pub field: String,
    /// Value recorded in the profile.
    pub trained: String,
    /// Live value.
    pub live: String,
    /// Severity of this signal.
    pub severity: DriftSeverity,
    /// Short note.
    pub note: String,
}

/// Actionable retrain recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrainRecommendation {
    /// Whether a retrain is recommended.
    pub recommended: bool,
    /// Severity driving the recommendation.
    pub severity: DriftSeverity,
    /// Human summary.
    pub summary: String,
    /// Suggested CLI command.
    pub suggested_command: String,
    /// Target names to retrain (empty = full profile).
    pub targets: Vec<String>,
    /// True when only a subset of targets is affected.
    pub partial: bool,
}

/// Full staleness report for doctor / verify.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessReport {
    /// Profile path or label.
    pub profile_label: String,
    /// Profile fingerprint.
    pub fingerprint: String,
    /// Live fingerprint when known.
    pub live_fingerprint: Option<String>,
    /// Fingerprint exact match (when live fingerprint provided).
    pub fingerprint_match: Option<bool>,
    /// Aggregate severity (max of signals).
    pub severity: DriftSeverity,
    /// Individual signals.
    pub signals: Vec<DriftSignal>,
    /// Retrain recommendation.
    pub retrain: RetrainRecommendation,
}

/// Policy knobs for staleness checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessPolicy {
    /// Soft-warn after this many days (still usable).
    pub soft_age_days: i64,
    /// Hard-stale after this many days.
    pub hard_age_days: i64,
    /// Treat logical CPU count change as soft drift.
    pub watch_logical_cpus: bool,
}

impl Default for StalenessPolicy {
    fn default() -> Self {
        Self {
            soft_age_days: 14,
            hard_age_days: 90,
            watch_logical_cpus: true,
        }
    }
}

fn max_severity(a: DriftSeverity, b: DriftSeverity) -> DriftSeverity {
    use DriftSeverity::*;
    match (a, b) {
        (Hard, _) | (_, Hard) => Hard,
        (Soft, _) | (_, Soft) => Soft,
        _ => None,
    }
}

/// Assess staleness of `profile` against a live environment snapshot.
pub fn assess_staleness(
    profile: &HnepProfile,
    live: &EnvironmentSnapshot,
    live_fingerprint: Option<&str>,
    policy: &StalenessPolicy,
) -> StalenessReport {
    let mut signals = Vec::new();

    if profile.environment.os != live.os {
        signals.push(DriftSignal {
            field: "os".into(),
            trained: profile.environment.os.clone(),
            live: live.os.clone(),
            severity: DriftSeverity::Hard,
            note: "OS family changed; specialization may not transfer".into(),
        });
    }
    if profile.environment.arch != live.arch {
        signals.push(DriftSignal {
            field: "arch".into(),
            trained: profile.environment.arch.clone(),
            live: live.arch.clone(),
            severity: DriftSeverity::Hard,
            note: "Architecture changed; full retrain required".into(),
        });
    }
    if policy.watch_logical_cpus && profile.environment.logical_cpus != live.logical_cpus {
        signals.push(DriftSignal {
            field: "logical_cpus".into(),
            trained: profile.environment.logical_cpus.to_string(),
            live: live.logical_cpus.to_string(),
            severity: DriftSeverity::Soft,
            note: "Logical CPU count drifted; concurrency / thread experiments may be stale"
                .into(),
        });
    }
    if !profile.environment.os_version.is_empty()
        && !live.os_version.is_empty()
        && profile.environment.os_version != live.os_version
    {
        signals.push(DriftSignal {
            field: "os_version".into(),
            trained: profile.environment.os_version.clone(),
            live: live.os_version.clone(),
            severity: DriftSeverity::Soft,
            note: "OS version string differs; soft refresh recommended".into(),
        });
    }

    if let (Ok(created), Ok(current)) = (
        chrono::DateTime::parse_from_rfc3339(&profile.header.created_at),
        chrono::DateTime::parse_from_rfc3339(&live.captured_at),
    ) {
        let age = current.signed_duration_since(created).num_days();
        if age > policy.hard_age_days {
            signals.push(DriftSignal {
                field: "age_days".into(),
                trained: profile.header.created_at.clone(),
                live: format!("{age} days"),
                severity: DriftSeverity::Hard,
                note: format!(
                    "Profile older than {} days; full retrain recommended",
                    policy.hard_age_days
                ),
            });
        } else if age > policy.soft_age_days {
            signals.push(DriftSignal {
                field: "age_days".into(),
                trained: profile.header.created_at.clone(),
                live: format!("{age} days"),
                severity: DriftSeverity::Soft,
                note: format!(
                    "Profile older than {} days; refresh when convenient",
                    policy.soft_age_days
                ),
            });
        }
    }

    let fingerprint_match = live_fingerprint.map(|fp| fp == profile.header.fingerprint);
    if fingerprint_match == Some(false) {
        signals.push(DriftSignal {
            field: "fingerprint".into(),
            trained: profile.header.fingerprint.clone(),
            live: live_fingerprint.unwrap_or("?").into(),
            severity: DriftSeverity::Hard,
            note: "Fingerprint mismatch — wrong machine or topology class changed".into(),
        });
    }

    let severity = signals
        .iter()
        .fold(DriftSeverity::None, |acc, s| max_severity(acc, s.severity));

    let retrain = build_retrain_recommendation(profile, &signals, severity);

    StalenessReport {
        profile_label: profile.header.label.clone(),
        fingerprint: profile.header.fingerprint.clone(),
        live_fingerprint: live_fingerprint.map(|s| s.to_string()),
        fingerprint_match,
        severity,
        signals,
        retrain,
    }
}

fn build_retrain_recommendation(
    profile: &HnepProfile,
    signals: &[DriftSignal],
    severity: DriftSeverity,
) -> RetrainRecommendation {
    if severity == DriftSeverity::None {
        return RetrainRecommendation {
            recommended: false,
            severity,
            summary: "Profile environment matches live host within policy".into(),
            suggested_command: String::new(),
            targets: Vec::new(),
            partial: false,
        };
    }

    let hard = severity == DriftSeverity::Hard;
    let only_cpus = signals.iter().all(|s| s.field == "logical_cpus")
        && signals.iter().any(|s| s.field == "logical_cpus");
    let only_soft_age = signals.iter().all(|s| s.field == "age_days")
        && !signals.iter().any(|s| s.severity == DriftSeverity::Hard);

    if only_cpus {
        let mut targets: Vec<String> = profile
            .workloads
            .iter()
            .filter(|w| w.name.contains("concurrency") || w.name.contains("thread"))
            .map(|w| w.name.clone())
            .collect();
        if !targets.iter().any(|t| t == "concurrency") {
            targets.push("concurrency".into());
        }
        return RetrainRecommendation {
            recommended: true,
            severity,
            summary: "Logical CPU count drifted — partial retrain of concurrency targets"
                .into(),
            suggested_command: format!(
                "silicera train --output <path> --only {}",
                targets.join(",")
            ),
            targets,
            partial: true,
        };
    }

    if hard {
        RetrainRecommendation {
            recommended: true,
            severity,
            summary: "Hard environment drift — full retrain recommended before trusting winners"
                .into(),
            suggested_command: "silicera train --output out/profile.hnep".into(),
            targets: Vec::new(),
            partial: false,
        }
    } else if only_soft_age {
        RetrainRecommendation {
            recommended: true,
            severity,
            summary: "Profile aging past soft threshold — refresh when convenient".into(),
            suggested_command: "silicera train --output out/profile.hnep".into(),
            targets: Vec::new(),
            partial: false,
        }
    } else {
        RetrainRecommendation {
            recommended: true,
            severity,
            summary: "Soft environment drift detected — refresh recommended".into(),
            suggested_command: "silicera train --output out/profile.hnep".into(),
            targets: Vec::new(),
            partial: false,
        }
    }
}
