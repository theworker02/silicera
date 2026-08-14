//! Partial retrain dependency tracking.
//!
//! Maps HNEP targets (workloads / size-classes) to topology and environment
//! dependencies so operators can re-measure only what drifted — without theater.

use serde::{Deserialize, Serialize};

use crate::hnep::HnepProfile;
use crate::staleness::{DriftSignal, DriftSeverity};

/// What a target depends on for its measured winner to remain meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrainDep {
    /// Fingerprint / microarch class.
    Fingerprint,
    /// L1D geometry.
    L1Geometry,
    /// L2 geometry.
    L2Geometry,
    /// L3 geometry.
    L3Geometry,
    /// Logical CPU / thread count.
    ThreadTopology,
    /// OS / arch environment.
    Environment,
    /// Age / soft refresh (everything eventually).
    Time,
}

impl RetrainDep {
    /// Stable id.
    pub fn id(self) -> &'static str {
        match self {
            RetrainDep::Fingerprint => "fingerprint",
            RetrainDep::L1Geometry => "l1_geometry",
            RetrainDep::L2Geometry => "l2_geometry",
            RetrainDep::L3Geometry => "l3_geometry",
            RetrainDep::ThreadTopology => "thread_topology",
            RetrainDep::Environment => "environment",
            RetrainDep::Time => "time",
        }
    }
}

/// One trainable target and its dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDeps {
    /// Target name (workload or `size_class:<CLASS>` or `decision_tree`).
    pub name: String,
    /// Kind: `workload`, `size_class`, `tree`.
    pub kind: String,
    /// Dependencies.
    pub deps: Vec<RetrainDep>,
}

/// Plan describing which targets need retrain given drift signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialRetrainPlan {
    /// Targets that must be re-measured.
    pub must_retrain: Vec<String>,
    /// Targets that can be kept.
    pub keep: Vec<String>,
    /// True when the plan is a proper subset (not full retrain).
    pub is_partial: bool,
    /// Human summary.
    pub summary: String,
    /// Suggested CLI fragment (`--only a,b,c` or empty for full).
    pub only_flag: String,
}

/// Build the dependency graph for a profile's targets.
pub fn target_dependencies(profile: &HnepProfile) -> Vec<TargetDeps> {
    let mut out = Vec::new();
    for w in &profile.workloads {
        out.push(TargetDeps {
            name: w.name.clone(),
            kind: "workload".into(),
            deps: deps_for_name(&w.name),
        });
    }
    for sc in &profile.size_classes {
        let name = format!("size_class:{}", sc.class);
        out.push(TargetDeps {
            name: name.clone(),
            kind: "size_class".into(),
            deps: deps_for_size_class(&sc.class),
        });
    }
    if profile.decision_tree.is_some() {
        out.push(TargetDeps {
            name: "decision_tree".into(),
            kind: "tree".into(),
            deps: vec![
                RetrainDep::L1Geometry,
                RetrainDep::L2Geometry,
                RetrainDep::L3Geometry,
                RetrainDep::Fingerprint,
            ],
        });
    }
    out
}

fn deps_for_name(name: &str) -> Vec<RetrainDep> {
    let n = name.to_ascii_lowercase();
    let mut deps = vec![RetrainDep::Fingerprint, RetrainDep::Environment];
    if n.contains("mem") || n.contains("l1") || n.contains("l2") || n.contains("l3") || n.contains("dram")
    {
        deps.push(RetrainDep::L1Geometry);
        deps.push(RetrainDep::L2Geometry);
        deps.push(RetrainDep::L3Geometry);
    }
    if n.contains("concurrency") || n.contains("thread") {
        deps.push(RetrainDep::ThreadTopology);
    }
    deps.push(RetrainDep::Time);
    deps
}

fn deps_for_size_class(class: &str) -> Vec<RetrainDep> {
    let mut deps = vec![RetrainDep::Fingerprint, RetrainDep::Environment, RetrainDep::Time];
    match class.to_ascii_uppercase().as_str() {
        "L1" => deps.push(RetrainDep::L1Geometry),
        "L2" => {
            deps.push(RetrainDep::L1Geometry);
            deps.push(RetrainDep::L2Geometry);
        }
        "L3" => {
            deps.push(RetrainDep::L1Geometry);
            deps.push(RetrainDep::L2Geometry);
            deps.push(RetrainDep::L3Geometry);
        }
        "DRAM" => {
            deps.push(RetrainDep::L3Geometry);
        }
        _ => {
            deps.push(RetrainDep::L1Geometry);
            deps.push(RetrainDep::L2Geometry);
            deps.push(RetrainDep::L3Geometry);
        }
    }
    deps
}

fn deps_triggered_by_signals(signals: &[DriftSignal]) -> Vec<RetrainDep> {
    let mut out = Vec::new();
    for s in signals {
        match s.field.as_str() {
            "fingerprint" => out.push(RetrainDep::Fingerprint),
            "os" | "arch" | "os_version" => out.push(RetrainDep::Environment),
            "logical_cpus" => out.push(RetrainDep::ThreadTopology),
            "age_days" => out.push(RetrainDep::Time),
            _ => {}
        }
        if s.severity == DriftSeverity::Hard && s.field == "fingerprint" {
            // Hard fingerprint mismatch invalidates geometry assumptions too.
            out.push(RetrainDep::L1Geometry);
            out.push(RetrainDep::L2Geometry);
            out.push(RetrainDep::L3Geometry);
        }
    }
    out.sort_by_key(|d| d.id());
    out.dedup();
    out
}

/// Compute a partial retrain plan from profile + staleness signals.
pub fn plan_partial_retrain(profile: &HnepProfile, signals: &[DriftSignal]) -> PartialRetrainPlan {
    let graph = target_dependencies(profile);
    if signals.is_empty() {
        return PartialRetrainPlan {
            must_retrain: Vec::new(),
            keep: graph.iter().map(|t| t.name.clone()).collect(),
            is_partial: false,
            summary: "No drift signals — keep existing measured winners".into(),
            only_flag: String::new(),
        };
    }

    let triggered = deps_triggered_by_signals(signals);
    // Hard OS/arch/fingerprint → full retrain.
    let force_full = signals.iter().any(|s| {
        s.severity == DriftSeverity::Hard
            && matches!(s.field.as_str(), "os" | "arch" | "fingerprint")
    });

    if force_full || triggered.contains(&RetrainDep::Time) && signals.iter().any(|s| {
        s.field == "age_days" && s.severity == DriftSeverity::Hard
    }) {
        let names: Vec<String> = graph.iter().map(|t| t.name.clone()).collect();
        return PartialRetrainPlan {
            must_retrain: names,
            keep: Vec::new(),
            is_partial: false,
            summary: "Hard drift — full retrain of all targets".into(),
            only_flag: String::new(),
        };
    }

    let mut must = Vec::new();
    let mut keep = Vec::new();
    for t in &graph {
        let hit = t.deps.iter().any(|d| triggered.contains(d));
        if hit {
            must.push(t.name.clone());
        } else {
            keep.push(t.name.clone());
        }
    }

    // Soft age alone → recommend full refresh but mark keep empty for honesty.
    if must.is_empty() && triggered.contains(&RetrainDep::Time) {
        let names: Vec<String> = graph.iter().map(|t| t.name.clone()).collect();
        return PartialRetrainPlan {
            must_retrain: names,
            keep: Vec::new(),
            is_partial: false,
            summary: "Age soft-threshold — prefer full refresh".into(),
            only_flag: String::new(),
        };
    }

    let is_partial = !must.is_empty() && !keep.is_empty();
    let only_flag = if is_partial {
        // Map size_class:L1 → memscan-l1 style names for train --only.
        let mapped: Vec<String> = must
            .iter()
            .map(|n| {
                if let Some(rest) = n.strip_prefix("size_class:") {
                    format!("memscan-{}", rest.to_ascii_lowercase())
                } else if n == "decision_tree" {
                    "size_classes".into()
                } else {
                    n.clone()
                }
            })
            .collect();
        format!("--only {}", mapped.join(","))
    } else {
        String::new()
    };

    PartialRetrainPlan {
        summary: if is_partial {
            format!(
                "Partial retrain: {} target(s) affected, {} kept",
                must.len(),
                keep.len()
            )
        } else if must.is_empty() {
            "No targets selected for retrain".into()
        } else {
            "Full retrain of all targets".into()
        },
        must_retrain: must,
        keep,
        is_partial,
        only_flag,
    }
}

/// Parse a comma-separated `--only` filter into target name predicates.
pub fn parse_only_filter(only: &str) -> Vec<String> {
    only.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Whether a tournament / size-class name matches an `--only` filter entry.
pub fn target_matches_filter(name: &str, filter: &[String]) -> bool {
    if filter.is_empty() {
        return true;
    }
    let n = name.to_ascii_lowercase();
    filter.iter().any(|f| {
        let f = f.to_ascii_lowercase();
        n == f
            || n.contains(&f)
            || f == "size_classes" && n.starts_with("memscan-")
            || f == "all"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::Fingerprint;
    use crate::hardware::EnvironmentSnapshot;
    use crate::hnep::{Confidence, SizeClassEntry, WorkloadEntry};
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
            "test",
        )
        .unwrap();
        p.workloads.push(WorkloadEntry {
            name: "concurrency".into(),
            winner: "candidate".into(),
            confidence: Confidence::Medium,
            rationale: "test".into(),
            winner_median_ns: Some(1.0),
            baseline_median_ns: Some(2.0),
        });
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
    fn cpu_drift_selects_concurrency() {
        let p = tiny_profile();
        let signals = vec![DriftSignal {
            field: "logical_cpus".into(),
            trained: "32".into(),
            live: "16".into(),
            severity: DriftSeverity::Soft,
            note: "cpus".into(),
        }];
        let plan = plan_partial_retrain(&p, &signals);
        assert!(plan.is_partial);
        assert!(plan.must_retrain.iter().any(|t| t.contains("concurrency")));
        assert!(plan.keep.iter().any(|t| t == "integer"));
    }

    #[test]
    fn filter_matching() {
        let f = parse_only_filter("integer,memscan-l1");
        assert!(target_matches_filter("integer", &f));
        assert!(target_matches_filter("memscan-l1", &f));
        assert!(!target_matches_filter("float", &f));
        assert!(target_matches_filter("memscan-l2", &parse_only_filter("size_classes")));
    }
}
