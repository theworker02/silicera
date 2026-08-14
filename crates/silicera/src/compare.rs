//! Cross-profile comparison for Silicon Split and HNEP side-by-side analysis.
//!
//! Comparison is structural and statistical over **already-measured** artifacts.
//! It never invents Machine B timings. When only one side is present, the
//! answer to “do two machines benefit from different strategies?” is UNKNOWN.

use serde::{Deserialize, Serialize};

use crate::hnep::{Confidence, HnepProfile, SizeClassEntry};

/// Stable protocol id for the multi-machine Silicon Split experiment.
pub const SILICON_SPLIT_PROTOCOL: &str = "silicera-silicon-split/1";

/// Protocol version string embedded in sanitized exports.
pub const SILICON_SPLIT_PROTOCOL_VERSION: &str = "1.0.0";

/// How a single workload or size-class target compares across two profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DivergenceKind {
    /// Same winner id on both sides.
    Same,
    /// Different winners.
    Diverged,
    /// Present only on side A.
    OnlyA,
    /// Present only on side B.
    OnlyB,
    /// Side B slot is a placeholder (no measured profile).
    PlaceholderB,
}

impl DivergenceKind {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            DivergenceKind::Same => "SAME",
            DivergenceKind::Diverged => "DIVERGED",
            DivergenceKind::OnlyA => "ONLY_A",
            DivergenceKind::OnlyB => "ONLY_B",
            DivergenceKind::PlaceholderB => "PLACEHOLDER_B",
        }
    }
}

/// High-level answer to the Silicon Split research question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SplitVerdict {
    /// Both machines measured; at least one target has different winners with usable confidence.
    Yes,
    /// Both machines measured; winners match on all shared targets (or only inconclusive noise).
    No,
    /// Evidence too weak (all INCONCLUSIVE / missing medians).
    Inconclusive,
    /// Machine B not measured yet (placeholder or missing profile).
    Unknown,
}

impl SplitVerdict {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            SplitVerdict::Yes => "YES",
            SplitVerdict::No => "NO",
            SplitVerdict::Inconclusive => "INCONCLUSIVE",
            SplitVerdict::Unknown => "UNKNOWN",
        }
    }
}

/// One compared target (workload or size-class).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDelta {
    /// Target category: `workload` or `size_class`.
    pub kind: String,
    /// Target name (workload name or size-class id).
    pub name: String,
    /// Winner on A (if present).
    pub winner_a: Option<String>,
    /// Winner on B (if present).
    pub winner_b: Option<String>,
    /// Confidence on A.
    pub confidence_a: Option<Confidence>,
    /// Confidence on B.
    pub confidence_b: Option<Confidence>,
    /// Median ns on A (informational).
    pub median_a_ns: Option<f64>,
    /// Median ns on B (informational).
    pub median_b_ns: Option<f64>,
    /// Relative delta of winner medians: (b - a) / a, when both present.
    /// This is **not** a claimed speedup; it is a cross-machine median delta.
    pub median_rel_delta: Option<f64>,
    /// Divergence classification.
    pub divergence: DivergenceKind,
    /// Short note.
    pub note: String,
}

/// Strategy vector: ordered winner ids for a fixed candidate/workload set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVector {
    /// Side label (`A`, `B`, or custom).
    pub side: String,
    /// Fingerprint string when known.
    pub fingerprint: Option<String>,
    /// Workload → winner.
    pub workloads: Vec<(String, String)>,
    /// Size-class → winner.
    pub size_classes: Vec<(String, String)>,
}

impl StrategyVector {
    /// Build from an HNEP.
    pub fn from_profile(side: impl Into<String>, profile: &HnepProfile) -> Self {
        Self {
            side: side.into(),
            fingerprint: Some(profile.header.fingerprint.clone()),
            workloads: profile
                .workloads
                .iter()
                .map(|w| (w.name.clone(), w.winner.clone()))
                .collect(),
            size_classes: profile
                .size_classes
                .iter()
                .map(|s| (s.class.clone(), s.winner.clone()))
                .collect(),
        }
    }

    /// Compact display line.
    pub fn compact(&self) -> String {
        let mut parts: Vec<String> = self
            .workloads
            .iter()
            .map(|(n, w)| format!("{n}={w}"))
            .collect();
        for (n, w) in &self.size_classes {
            parts.push(format!("sc:{n}={w}"));
        }
        parts.join(", ")
    }
}

/// Full compare report (publishable structure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareReport {
    /// Protocol id when this is a Silicon Split compare.
    pub protocol: String,
    /// Fingerprint A.
    pub fingerprint_a: String,
    /// Fingerprint B (empty / placeholder marker when absent).
    pub fingerprint_b: String,
    /// Whether fingerprints are equal.
    pub fingerprints_equal: bool,
    /// Strategy vector A.
    pub strategy_a: StrategyVector,
    /// Strategy vector B (may be empty placeholder).
    pub strategy_b: StrategyVector,
    /// Per-target deltas.
    pub targets: Vec<TargetDelta>,
    /// Count of DIVERGED targets.
    pub diverged_count: usize,
    /// Count of SAME targets.
    pub same_count: usize,
    /// Overall Silicon Split verdict.
    pub verdict: SplitVerdict,
    /// Human-readable summary (no fabricated %).
    pub summary: String,
    /// True when B was a schema placeholder, not a measured profile.
    pub b_is_placeholder: bool,
}

/// Compare two measured HNEP profiles under an identical protocol assumption.
pub fn compare_profiles(a: &HnepProfile, b: &HnepProfile) -> CompareReport {
    let mut targets = Vec::new();

    for wa in &a.workloads {
        let wb = b.workloads.iter().find(|x| x.name == wa.name);
        targets.push(match wb {
            Some(wb) => {
                let diverged = wa.winner != wb.winner;
                let median_rel = relative_delta(wa.winner_median_ns, wb.winner_median_ns);
                TargetDelta {
                    kind: "workload".into(),
                    name: wa.name.clone(),
                    winner_a: Some(wa.winner.clone()),
                    winner_b: Some(wb.winner.clone()),
                    confidence_a: Some(wa.confidence),
                    confidence_b: Some(wb.confidence),
                    median_a_ns: wa.winner_median_ns,
                    median_b_ns: wb.winner_median_ns,
                    median_rel_delta: median_rel,
                    divergence: if diverged {
                        DivergenceKind::Diverged
                    } else {
                        DivergenceKind::Same
                    },
                    note: if diverged {
                        format!(
                            "winners differ (A conf={}, B conf={})",
                            wa.confidence.label(),
                            wb.confidence.label()
                        )
                    } else {
                        "same winner".into()
                    },
                }
            }
            None => TargetDelta {
                kind: "workload".into(),
                name: wa.name.clone(),
                winner_a: Some(wa.winner.clone()),
                winner_b: None,
                confidence_a: Some(wa.confidence),
                confidence_b: None,
                median_a_ns: wa.winner_median_ns,
                median_b_ns: None,
                median_rel_delta: None,
                divergence: DivergenceKind::OnlyA,
                note: "target only in A".into(),
            },
        });
    }
    for wb in &b.workloads {
        if !a.workloads.iter().any(|x| x.name == wb.name) {
            targets.push(TargetDelta {
                kind: "workload".into(),
                name: wb.name.clone(),
                winner_a: None,
                winner_b: Some(wb.winner.clone()),
                confidence_a: None,
                confidence_b: Some(wb.confidence),
                median_a_ns: None,
                median_b_ns: wb.winner_median_ns,
                median_rel_delta: None,
                divergence: DivergenceKind::OnlyB,
                note: "target only in B".into(),
            });
        }
    }

    for sa in &a.size_classes {
        let sb = b.size_classes.iter().find(|x| x.class == sa.class);
        targets.push(size_class_delta(sa, sb));
    }
    for sb in &b.size_classes {
        if !a.size_classes.iter().any(|x| x.class == sb.class) {
            targets.push(TargetDelta {
                kind: "size_class".into(),
                name: sb.class.clone(),
                winner_a: None,
                winner_b: Some(sb.winner.clone()),
                confidence_a: None,
                confidence_b: Some(sb.confidence),
                median_a_ns: None,
                median_b_ns: sb.winner_median_ns,
                median_rel_delta: None,
                divergence: DivergenceKind::OnlyB,
                note: "size class only in B".into(),
            });
        }
    }

    finalize_report(a, b, targets, false)
}

/// Compare a measured Machine A profile against a Machine B **placeholder**.
///
/// Does not invent B numbers. Verdict is always UNKNOWN.
pub fn compare_with_placeholder(a: &HnepProfile, placeholder_label: &str) -> CompareReport {
    let mut targets = Vec::new();
    for wa in &a.workloads {
        targets.push(TargetDelta {
            kind: "workload".into(),
            name: wa.name.clone(),
            winner_a: Some(wa.winner.clone()),
            winner_b: None,
            confidence_a: Some(wa.confidence),
            confidence_b: None,
            median_a_ns: wa.winner_median_ns,
            median_b_ns: None,
            median_rel_delta: None,
            divergence: DivergenceKind::PlaceholderB,
            note: format!("Machine B slot '{placeholder_label}' not measured"),
        });
    }
    for sa in &a.size_classes {
        targets.push(TargetDelta {
            kind: "size_class".into(),
            name: sa.class.clone(),
            winner_a: Some(sa.winner.clone()),
            winner_b: None,
            confidence_a: Some(sa.confidence),
            confidence_b: None,
            median_a_ns: sa.winner_median_ns,
            median_b_ns: None,
            median_rel_delta: None,
            divergence: DivergenceKind::PlaceholderB,
            note: format!("Machine B size-class '{placeholder_label}' not measured"),
        });
    }

    let strategy_a = StrategyVector::from_profile("A", a);
    let strategy_b = StrategyVector {
        side: "B".into(),
        fingerprint: None,
        workloads: Vec::new(),
        size_classes: Vec::new(),
    };
    let placeholder_count = targets.len();
    CompareReport {
        protocol: SILICON_SPLIT_PROTOCOL.into(),
        fingerprint_a: a.header.fingerprint.clone(),
        fingerprint_b: format!("PLACEHOLDER:{placeholder_label}"),
        fingerprints_equal: false,
        strategy_a,
        strategy_b,
        targets,
        diverged_count: 0,
        same_count: 0,
        verdict: SplitVerdict::Unknown,
        summary: format!(
            "Machine A measured ({placeholder_count} targets). Machine B is a placeholder — \
             verdict UNKNOWN until a second Zen host trains with the same protocol. \
             Do not invent Machine B winners or medians."
        ),
        b_is_placeholder: true,
    }
}

fn size_class_delta(sa: &SizeClassEntry, sb: Option<&SizeClassEntry>) -> TargetDelta {
    match sb {
        Some(sb) => {
            let diverged = sa.winner != sb.winner;
            TargetDelta {
                kind: "size_class".into(),
                name: sa.class.clone(),
                winner_a: Some(sa.winner.clone()),
                winner_b: Some(sb.winner.clone()),
                confidence_a: Some(sa.confidence),
                confidence_b: Some(sb.confidence),
                median_a_ns: sa.winner_median_ns,
                median_b_ns: sb.winner_median_ns,
                median_rel_delta: relative_delta(sa.winner_median_ns, sb.winner_median_ns),
                divergence: if diverged {
                    DivergenceKind::Diverged
                } else {
                    DivergenceKind::Same
                },
                note: if diverged {
                    format!(
                        "size-class winners differ (A={}, B={})",
                        sa.confidence.label(),
                        sb.confidence.label()
                    )
                } else {
                    "same size-class winner".into()
                },
            }
        }
        None => TargetDelta {
            kind: "size_class".into(),
            name: sa.class.clone(),
            winner_a: Some(sa.winner.clone()),
            winner_b: None,
            confidence_a: Some(sa.confidence),
            confidence_b: None,
            median_a_ns: sa.winner_median_ns,
            median_b_ns: None,
            median_rel_delta: None,
            divergence: DivergenceKind::OnlyA,
            note: "size class only in A".into(),
        },
    }
}

fn relative_delta(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) if a.abs() > f64::EPSILON => Some((b - a) / a),
        _ => None,
    }
}

fn finalize_report(
    a: &HnepProfile,
    b: &HnepProfile,
    targets: Vec<TargetDelta>,
    b_is_placeholder: bool,
) -> CompareReport {
    let diverged_count = targets
        .iter()
        .filter(|t| t.divergence == DivergenceKind::Diverged)
        .count();
    let same_count = targets
        .iter()
        .filter(|t| t.divergence == DivergenceKind::Same)
        .count();

    let verdict = if b_is_placeholder {
        SplitVerdict::Unknown
    } else {
        let strong_diverge = targets.iter().any(|t| {
            t.divergence == DivergenceKind::Diverged
                && t.confidence_a.map(|c| c.rank()).unwrap_or(0) >= Confidence::Medium.rank()
                && t.confidence_b.map(|c| c.rank()).unwrap_or(0) >= Confidence::Medium.rank()
        });
        let any_diverge = diverged_count > 0;
        let all_weak = targets.iter().all(|t| {
            matches!(
                t.confidence_a,
                Some(Confidence::Inconclusive) | Some(Confidence::Low) | None
            ) && matches!(
                t.confidence_b,
                Some(Confidence::Inconclusive) | Some(Confidence::Low) | None
            )
        });
        if strong_diverge {
            SplitVerdict::Yes
        } else if any_diverge && !all_weak {
            SplitVerdict::Yes
        } else if all_weak && targets.is_empty() == false && diverged_count == 0 && same_count == 0
        {
            SplitVerdict::Inconclusive
        } else if diverged_count == 0 && same_count > 0 {
            if all_weak {
                SplitVerdict::Inconclusive
            } else {
                SplitVerdict::No
            }
        } else if any_diverge {
            SplitVerdict::Inconclusive
        } else {
            SplitVerdict::Inconclusive
        }
    };

    let summary = match verdict {
        SplitVerdict::Yes => format!(
            "{diverged_count} target(s) diverged with usable confidence — machines may benefit \
             from different measured strategies. Inspect per-target rows; do not over-generalize."
        ),
        SplitVerdict::No => format!(
            "Shared targets agree on winners ({same_count} SAME). No evidence yet that these two \
             machines need different strategies for this protocol."
        ),
        SplitVerdict::Inconclusive => {
            "Comparison completed but evidence is weak (low/inconclusive confidence or noisy \
             medians). Re-run with more iterations before claiming divergence."
                .into()
        }
        SplitVerdict::Unknown => {
            "Machine B not measured. Verdict UNKNOWN.".into()
        }
    };

    CompareReport {
        protocol: SILICON_SPLIT_PROTOCOL.into(),
        fingerprint_a: a.header.fingerprint.clone(),
        fingerprint_b: b.header.fingerprint.clone(),
        fingerprints_equal: a.header.fingerprint == b.header.fingerprint,
        strategy_a: StrategyVector::from_profile("A", a),
        strategy_b: StrategyVector::from_profile("B", b),
        targets,
        diverged_count,
        same_count,
        verdict,
        summary,
        b_is_placeholder,
    }
}

/// Sanitized export of an HNEP for cross-machine Silicon Split exchange.
///
/// Strips nothing security-sensitive beyond what HNEP already stores (no serials),
/// but adds explicit protocol metadata so Machine B can refuse mismatched protocols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedSplitExport {
    /// Protocol id.
    pub protocol: String,
    /// Protocol semver.
    pub protocol_version: String,
    /// Machine role hint (`A` or `B`).
    pub machine_role: String,
    /// Embedded HNEP (integrity still verified independently).
    pub profile: HnepProfile,
    /// Strategy vector snapshot.
    pub strategy_vector: StrategyVector,
    /// Measurement protocol echo (warmup/iterations/min_improvement) when known.
    pub measurement_echo: MeasurementEcho,
}

/// Echo of measurement knobs used during train (for protocol identity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementEcho {
    /// Warmup iterations.
    pub warmup: usize,
    /// Timed iterations.
    pub iterations: usize,
    /// Minimum relative improvement used in tournaments.
    pub min_improvement: f64,
    /// Candidate set id (stable string naming the variant pair set).
    pub candidate_set: String,
    /// Workload set id.
    pub workload_set: String,
}

impl Default for MeasurementEcho {
    fn default() -> Self {
        Self {
            warmup: 3,
            iterations: 20,
            min_improvement: 0.03,
            candidate_set: "baseline|candidate|prefetch|scan|copy".into(),
            workload_set: "memscan-size-classes|integer|float|branch".into(),
        }
    }
}

/// Machine B placeholder schema (no invented numbers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineBPlaceholder {
    /// Schema id.
    pub schema: String,
    /// Schema version.
    pub schema_version: String,
    /// Protocol that Machine B must run.
    pub protocol: String,
    /// Instructions for completing Machine B.
    pub instructions: Vec<String>,
    /// Required artifact paths (relative suggestions).
    pub required_artifacts: Vec<String>,
    /// Fields that must appear in Machine B’s sanitized export.
    pub required_fields: Vec<String>,
    /// Explicit non-goals.
    pub do_not: Vec<String>,
    /// Machine A fingerprint for reference (not a B measurement).
    pub machine_a_fingerprint: Option<String>,
    /// Machine A strategy vector (reference only).
    pub machine_a_strategy_reference: Option<StrategyVector>,
}

impl MachineBPlaceholder {
    /// Build a placeholder document referencing Machine A’s measured profile.
    pub fn from_machine_a(a: &HnepProfile) -> Self {
        Self {
            schema: "silicera-machine-b-placeholder".into(),
            schema_version: "1.0.0".into(),
            protocol: SILICON_SPLIT_PROTOCOL.into(),
            instructions: vec![
                "On a second AMD Zen3/Zen4/Zen5 host, check out the same Silicera revision.".into(),
                "Run: silicera silicon-split train --role B -o out/machine_b.hnep".into(),
                "Export: silicera silicon-split export --profile out/machine_b.hnep --role B -o out/machine_b.split.json".into(),
                "Copy machine_b.split.json (or machine_b.hnep) back to Machine A.".into(),
                "Compare: silicera compare out/machine_a.hnep out/machine_b.hnep --json".into(),
                "Or: silicera silicon-split report --a out/machine_a.split.json --b out/machine_b.split.json".into(),
            ],
            required_artifacts: vec![
                "out/machine_b.hnep".into(),
                "out/machine_b.split.json".into(),
            ],
            required_fields: vec![
                "protocol".into(),
                "protocol_version".into(),
                "machine_role".into(),
                "profile.header.fingerprint".into(),
                "profile.workloads".into(),
                "profile.size_classes".into(),
                "profile.digest".into(),
                "strategy_vector".into(),
                "measurement_echo".into(),
            ],
            do_not: vec![
                "Do not invent Machine B medians, winners, or fingerprints.".into(),
                "Do not copy Machine A winners into the B slot.".into(),
                "Do not publish a YES/NO Silicon Split verdict until B is measured.".into(),
            ],
            machine_a_fingerprint: Some(a.header.fingerprint.clone()),
            machine_a_strategy_reference: Some(StrategyVector::from_profile("A", a)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::Fingerprint;
    use crate::hardware::EnvironmentSnapshot;
    use crate::hnep::{HnepHeader, IntegrityDigest, WorkloadEntry};
    use crate::knowledge::Microarch;
    use crate::topology::TopologyGraph;

    fn profile_with(fp: &str, workloads: Vec<WorkloadEntry>) -> HnepProfile {
        let header = HnepHeader {
            format: crate::hnep::HNEP_FORMAT.into(),
            version: crate::hnep::HNEP_VERSION,
            silicera_version: crate::VERSION.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            fingerprint: fp.into(),
            label: "test".into(),
        };
        let environment = EnvironmentSnapshot::capture();
        let size_classes = Vec::new();
        #[derive(serde::Serialize)]
        struct Payload {
            header: HnepHeader,
            environment: EnvironmentSnapshot,
            workloads: Vec<WorkloadEntry>,
            size_classes: Vec<SizeClassEntry>,
            decision_tree: Option<crate::specialize::DecisionTree>,
        }
        let payload = Payload {
            header: header.clone(),
            environment: environment.clone(),
            workloads: workloads.clone(),
            size_classes: size_classes.clone(),
            decision_tree: None,
        };
        let digest = IntegrityDigest::sha256(&serde_json::to_vec(&payload).unwrap());
        HnepProfile {
            header,
            environment,
            workloads,
            size_classes,
            decision_tree: None,
            digest,
        }
    }

    #[test]
    fn diverge_yes_when_winners_differ() {
        let a = profile_with(
            "SLC:AMD:ZEN5:1A:44:00:aaaaaaaaaaaaaaaa:bbbbbbbbbbbbbbbb",
            vec![WorkloadEntry {
                name: "memscan".into(),
                winner: "scan".into(),
                confidence: Confidence::High,
                rationale: "a".into(),
                winner_median_ns: Some(100.0),
                baseline_median_ns: Some(120.0),
            }],
        );
        let b = profile_with(
            "SLC:AMD:ZEN4:19:61:00:cccccccccccccccc:dddddddddddddddd",
            vec![WorkloadEntry {
                name: "memscan".into(),
                winner: "copy".into(),
                confidence: Confidence::High,
                rationale: "b".into(),
                winner_median_ns: Some(110.0),
                baseline_median_ns: Some(120.0),
            }],
        );
        let report = compare_profiles(&a, &b);
        assert_eq!(report.verdict, SplitVerdict::Yes);
        assert_eq!(report.diverged_count, 1);
        assert!(!report.b_is_placeholder);
    }

    #[test]
    fn placeholder_is_unknown() {
        let fp = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &TopologyGraph::new());
        let a = profile_with(
            &fp.value,
            vec![WorkloadEntry {
                name: "integer".into(),
                winner: "baseline".into(),
                confidence: Confidence::Inconclusive,
                rationale: "t".into(),
                winner_median_ns: Some(50.0),
                baseline_median_ns: Some(50.0),
            }],
        );
        let report = compare_with_placeholder(&a, "second-zen-box");
        assert_eq!(report.verdict, SplitVerdict::Unknown);
        assert!(report.b_is_placeholder);
        assert!(report.targets.iter().all(|t| t.divergence == DivergenceKind::PlaceholderB));
    }

    #[test]
    fn same_winners_no() {
        let w = WorkloadEntry {
            name: "integer".into(),
            winner: "baseline".into(),
            confidence: Confidence::Medium,
            rationale: "t".into(),
            winner_median_ns: Some(50.0),
            baseline_median_ns: Some(50.0),
        };
        let a = profile_with("fp-a", vec![w.clone()]);
        let b = profile_with("fp-b", vec![w]);
        let report = compare_profiles(&a, &b);
        assert_eq!(report.verdict, SplitVerdict::No);
        assert_eq!(report.same_count, 1);
    }
}
