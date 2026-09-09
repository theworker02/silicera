//! Deterministic profile revision diffs, without inferring performance claims.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::{HnepProfile, Result, SiliceraError};

/// One changed field. Missing sides denote additions or removals.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileChange {
    /// `added`, `removed`, or `modified`; distinguishes absence from JSON null.
    pub kind: &'static str,
    /// JSON pointer with workload names and size-class IDs replacing indexes.
    pub path: String,
    /// Previous value, absent for additions.
    pub before: Option<Value>,
    /// New value, absent for removals.
    pub after: Option<Value>,
}

/// Review of two verified profiles. Timing changes are observations only.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileDiff {
    /// Stable report format identifier.
    pub format: &'static str,
    /// Whether the profiles target exactly the same fingerprint.
    pub same_fingerprint: bool,
    /// Sorted field changes, excluding creation timestamp and integrity digest.
    pub changes: Vec<ProfileChange>,
}

impl ProfileDiff {
    /// Compare verified artifacts, matching entries by identity rather than order.
    /// Duplicate identities are rejected because their dispatch meaning is ambiguous.
    pub fn between(before: &HnepProfile, after: &HnepProfile) -> Result<Self> {
        let before = HnepProfile::parse_str(&serde_json::to_string(before)?)?;
        let after = HnepProfile::parse_str(&serde_json::to_string(after)?)?;
        let a = fields(&before)?;
        let b = fields(&after)?;
        let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
        let changes = keys
            .into_iter()
            .filter_map(|path| {
                let old = a.get(path);
                let new = b.get(path);
                (old != new).then(|| ProfileChange {
                    kind: if old.is_none() {
                        "added"
                    } else if new.is_none() {
                        "removed"
                    } else {
                        "modified"
                    },
                    path: path.clone(),
                    before: old.cloned(),
                    after: new.cloned(),
                })
            })
            .collect();
        Ok(Self {
            format: "silicera-profile-diff/1",
            same_fingerprint: before.header.fingerprint == after.header.fingerprint,
            changes,
        })
    }
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn fields(profile: &HnepProfile) -> Result<BTreeMap<String, Value>> {
    let mut value = serde_json::to_value(profile)?;
    let root = value.as_object_mut().expect("profile serializes as object");
    root.remove("digest");
    root.get_mut("header")
        .and_then(Value::as_object_mut)
        .unwrap()
        .remove("created_at");
    for (collection, identity) in [("workloads", "name"), ("size_classes", "class")] {
        let entries = root.remove(collection).unwrap();
        let mut indexed = serde_json::Map::new();
        for entry in entries.as_array().unwrap() {
            let name = entry[identity].as_str().unwrap();
            if indexed.insert(name.into(), entry.clone()).is_some() {
                return Err(SiliceraError::Parse(format!(
                    "duplicate {collection} identity: {name}"
                )));
            }
        }
        root.insert(collection.into(), Value::Object(indexed));
    }
    let mut fields = BTreeMap::new();
    flatten("", &value, &mut fields);
    Ok(fields)
}

fn flatten(path: &str, value: &Value, fields: &mut BTreeMap<String, Value>) {
    if let Value::Object(object) = value {
        for (key, child) in object {
            flatten(&format!("{path}/{}", escape(key)), child, fields);
        }
    } else {
        fields.insert(path.into(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Confidence, EnvironmentSnapshot, HardwareBackend, KnowledgePack, MockHardware,
        WorkloadEntry,
    };

    fn profile() -> HnepProfile {
        let host = MockHardware::zen5_dual_ccd()
            .discover(&KnowledgePack::builtin())
            .unwrap();
        HnepProfile::from_tournaments(
            host.fingerprint.as_ref().unwrap(),
            EnvironmentSnapshot::capture(),
            &[],
            vec![],
            None,
            "test",
        )
        .unwrap()
    }

    fn entry(name: &str) -> WorkloadEntry {
        WorkloadEntry {
            name: name.into(),
            winner: "baseline".into(),
            confidence: Confidence::Medium,
            rationale: "test".into(),
            winner_median_ns: Some(10.0),
            baseline_median_ns: Some(10.0),
        }
    }

    #[test]
    fn ignores_order_and_timestamp_but_reports_winner_and_timing() {
        let mut a = profile();
        a.upsert_workload(entry("a/b")).unwrap();
        a.upsert_workload(entry("other")).unwrap();
        let mut b = a.clone();
        b.workloads.reverse();
        b.header.created_at = "2026-01-01T00:00:00Z".into();
        b.recompute_digest().unwrap();
        assert!(ProfileDiff::between(&a, &b).unwrap().changes.is_empty());
        b.workloads[1].winner = "fast".into();
        b.workloads[1].winner_median_ns = Some(8.0);
        b.recompute_digest().unwrap();
        let diff = ProfileDiff::between(&a, &b).unwrap();
        assert!(diff.same_fingerprint);
        assert_eq!(diff.changes.len(), 2);
        assert_eq!(diff.changes[0].path, "/workloads/a~1b/winner");
    }

    #[test]
    fn additions_removals_empty_and_invalid_inputs() {
        let a = profile();
        assert!(ProfileDiff::between(&a, &a).unwrap().changes.is_empty());
        let mut b = a.clone();
        b.upsert_workload(entry("added")).unwrap();
        assert!(ProfileDiff::between(&a, &b)
            .unwrap()
            .changes
            .iter()
            .all(|c| c.before.is_none()));
        assert!(ProfileDiff::between(&b, &a)
            .unwrap()
            .changes
            .iter()
            .all(|c| c.after.is_none()));
        b.workloads.push(entry("added"));
        assert!(ProfileDiff::between(&a, &b).is_err());
        b.recompute_digest().unwrap();
        assert!(ProfileDiff::between(&a, &b)
            .unwrap_err()
            .to_string()
            .contains("duplicate"));
    }

    #[test]
    fn reports_environment_and_tree_changes() {
        let a = profile();
        let mut b = a.clone();
        b.environment.logical_cpus += 1;
        b.decision_tree = Some(crate::DecisionTree::from_thresholds(
            10, 20, 30, "a", "b", "c", "d", "baseline",
        ));
        b.recompute_digest().unwrap();
        let diff = ProfileDiff::between(&a, &b).unwrap();
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path == "/environment/logical_cpus"));
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path.starts_with("/decision_tree/")));
    }
}
