//! Profile lifecycle: integrity, confidence, staleness, and retrain assessment.
//!
//! Unifies the “is this HNEP still trustworthy?” story for doctor / health /
//! verify surfaces. Underlying [`crate::staleness`] and [`crate::retrain`]
//! modules remain the source of drift and partial-retrain logic.

use serde::{Deserialize, Serialize};

use crate::hardware::EnvironmentSnapshot;
use crate::hnep::{Confidence, HnepProfile};
use crate::retrain::{plan_partial_retrain, PartialRetrainPlan};
use crate::staleness::{
    assess_staleness, DriftSeverity, StalenessPolicy, StalenessReport,
};
use crate::Result;

/// Letter grade for [`ProfileHealth::score`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HealthGrade {
    /// Score ≥ 90.
    A,
    /// Score ≥ 75.
    B,
    /// Score ≥ 60.
    C,
    /// Score ≥ 40.
    D,
    /// Score < 40.
    F,
}

impl HealthGrade {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            HealthGrade::A => "A",
            HealthGrade::B => "B",
            HealthGrade::C => "C",
            HealthGrade::D => "D",
            HealthGrade::F => "F",
        }
    }

    fn from_score(score: u8) -> Self {
        match score {
            90..=100 => HealthGrade::A,
            75..=89 => HealthGrade::B,
            60..=74 => HealthGrade::C,
            40..=59 => HealthGrade::D,
            _ => HealthGrade::F,
        }
    }
}

/// Composite health of a loaded HNEP against the live host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileHealth {
    /// Profile label / path hint.
    pub profile_label: String,
    /// Fingerprint recorded in the profile.
    pub fingerprint: String,
    /// Aggregate score 0–100 (integrity + confidence + staleness).
    pub score: u8,
    /// Letter grade.
    pub grade: HealthGrade,
    /// Whether the integrity digest verified.
    pub integrity_ok: bool,
    /// Overall confidence label from the profile.
    pub confidence: String,
    /// Staleness severity label.
    pub staleness: String,
    /// Whether a retrain is recommended.
    pub retrain_recommended: bool,
    /// Operator-facing notes (measured facts / policy outcomes only).
    pub notes: Vec<String>,
    /// Full staleness report (for JSON / tooling).
    pub staleness_report: StalenessReport,
    /// Partial retrain plan derived from drift signals.
    pub retrain_plan: PartialRetrainPlan,
}

impl ProfileHealth {
    /// Assess profile health against a live environment snapshot.
    pub fn assess(
        profile: &HnepProfile,
        live: &EnvironmentSnapshot,
        live_fingerprint: Option<&str>,
        policy: &StalenessPolicy,
    ) -> Result<Self> {
        let mut notes = Vec::new();
        let integrity_ok = match profile.verify_integrity() {
            Ok(()) => {
                notes.push("Integrity digest verified.".into());
                true
            }
            Err(e) => {
                notes.push(format!("Integrity check failed: {e}"));
                false
            }
        };

        let overall = profile.overall_confidence();
        let confidence = overall.label().to_string();
        notes.push(format!("Overall confidence: {confidence}"));

        let staleness_report =
            assess_staleness(profile, live, live_fingerprint, policy);
        let retrain_plan = plan_partial_retrain(profile, &staleness_report.signals);

        let mut score: i32 = 100;
        if !integrity_ok {
            score -= 45;
        }
        match overall {
            Confidence::High => {}
            Confidence::Medium => {
                score -= 10;
                notes.push("Medium confidence — treat winners as provisional.".into());
            }
            Confidence::Low => {
                score -= 25;
                notes.push("Low confidence — prefer retrain before production dispatch.".into());
            }
            Confidence::Inconclusive => {
                score -= 35;
                notes.push("Inconclusive confidence — specialization not trustworthy.".into());
            }
        }
        match staleness_report.severity {
            DriftSeverity::None => {
                notes.push("Environment matches live host within policy.".into());
            }
            DriftSeverity::Soft => {
                score -= 15;
                notes.push(staleness_report.retrain.summary.clone());
            }
            DriftSeverity::Hard => {
                score -= 40;
                notes.push(staleness_report.retrain.summary.clone());
            }
        }
        if retrain_plan.is_partial {
            notes.push(format!(
                "Partial retrain available: {}",
                retrain_plan.only_flag
            ));
        }

        let score = score.clamp(0, 100) as u8;
        let grade = HealthGrade::from_score(score);
        notes.push(format!("Health grade {grade:?} (score {score}/100)."));

        Ok(Self {
            profile_label: profile.header.label.clone(),
            fingerprint: profile.header.fingerprint.clone(),
            score,
            grade,
            integrity_ok,
            confidence,
            staleness: staleness_report.severity.label().into(),
            retrain_recommended: staleness_report.retrain.recommended,
            notes,
            staleness_report,
            retrain_plan,
        })
    }

    /// Convenience: load-free assess with default policy.
    pub fn assess_default(
        profile: &HnepProfile,
        live: &EnvironmentSnapshot,
        live_fingerprint: Option<&str>,
    ) -> Result<Self> {
        Self::assess(profile, live, live_fingerprint, &StalenessPolicy::default())
    }
}

/// Re-export staleness assessment for a one-stop lifecycle import.
pub use crate::staleness::assess_staleness as assess_profile_staleness;

/// Re-export partial retrain planning.
pub use crate::retrain::plan_partial_retrain as plan_profile_retrain;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::Fingerprint;
    use crate::hnep::{SizeClassEntry, WorkloadEntry};
    use crate::knowledge::Microarch;
    use crate::topology::TopologyGraph;

    fn tiny_profile() -> HnepProfile {
        let fp = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &TopologyGraph::new());
        let mut p = HnepProfile::from_tournaments(
            &fp,
            EnvironmentSnapshot::capture(),
            &[],
            vec![SizeClassEntry {
                class: "L1".into(),
                threshold_bytes: 32 * 1024,
                working_set_bytes: 16 * 1024,
                winner: "copy".into(),
                confidence: Confidence::High,
                rationale: "test".into(),
                winner_median_ns: Some(100.0),
                baseline_median_ns: Some(200.0),
            }],
            None,
            "health-test",
        )
        .unwrap();
        p.workloads.push(WorkloadEntry {
            name: "integer".into(),
            winner: "baseline".into(),
            confidence: Confidence::High,
            rationale: "test".into(),
            winner_median_ns: Some(1.0),
            baseline_median_ns: Some(1.0),
        });
        p.recompute_digest().unwrap();
        p
    }

    #[test]
    fn healthy_profile_scores_high() {
        let p = tiny_profile();
        let live = p.environment.clone();
        let health =
            ProfileHealth::assess_default(&p, &live, Some(&p.header.fingerprint)).unwrap();
        assert!(health.integrity_ok);
        assert!(health.score >= 90);
        assert_eq!(health.grade, HealthGrade::A);
        assert!(!health.retrain_recommended);
    }
}
