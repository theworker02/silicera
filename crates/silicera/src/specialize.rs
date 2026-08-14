//! Size-dependent specialization and decision-tree dispatch.
//!
//! Working-set size relative to L1/L2/L3 drives variant selection. The tree is
//! encoded in HNEP and evaluated cheaply at runtime.
//!
//! When measured [`crate::hnep::SizeClassEntry`] values are available, prefer
//! [`DecisionTree::from_size_classes`] so winners per class come from tournaments
//! rather than topology placeholders alone.

use serde::{Deserialize, Serialize};

use crate::hnep::SizeClassEntry;
use crate::topology::TopologyGraph;

/// Specialization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializeConfig {
    /// Prefer native-tuned variants when confidence ≥ this rank (0–3).
    pub min_confidence_rank: u8,
    /// Enable size-dependent branching.
    pub size_dependent: bool,
}

impl Default for SpecializeConfig {
    fn default() -> Self {
        Self {
            min_confidence_rank: 1,
            size_dependent: true,
        }
    }
}

/// Result of a specialization planning pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializeResult {
    /// Decision tree produced.
    pub tree: DecisionTree,
    /// Notes for `silicera explain`.
    pub notes: Vec<String>,
}

/// A node in the size-dependent decision tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecisionNode {
    /// Branch on working-set size (bytes).
    SizeBranch {
        /// Threshold in bytes.
        threshold_bytes: u64,
        /// Label for explain output.
        label: String,
        /// Taken when size < threshold.
        less: Box<DecisionNode>,
        /// Taken when size ≥ threshold.
        greater_or_equal: Box<DecisionNode>,
    },
    /// Select a named variant.
    Select {
        /// Variant id.
        variant: String,
    },
}

/// Decision tree rooted at a node, with cache geometry context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    /// Cache thresholds used to build the tree (bytes).
    pub l1_bytes: u64,
    /// L2 threshold.
    pub l2_bytes: u64,
    /// L3 threshold (per CCD or total — documented in notes).
    pub l3_bytes: u64,
    /// Root node.
    pub root: DecisionNode,
    /// Default / baseline variant if evaluation fails.
    pub fallback_variant: String,
}

impl DecisionTree {
    /// Build a default size-dependent tree from topology + named variants.
    ///
    /// - size < L1 → `l1_resident`
    /// - size < L2 → `l2_resident`
    /// - size < L3 → `l3_resident`
    /// - else → `dram_friendly`
    pub fn from_topology(
        topo: &TopologyGraph,
        l1_variant: &str,
        l2_variant: &str,
        l3_variant: &str,
        dram_variant: &str,
        fallback: &str,
    ) -> Self {
        let (l1, l2, l3) = topology_thresholds(topo);
        Self::from_thresholds(l1, l2, l3, l1_variant, l2_variant, l3_variant, dram_variant, fallback)
    }

    /// Build a tree from measured size-class entries (flagship memscan/copy path).
    ///
    /// Missing classes fall back to `fallback`. Thresholds prefer entry
    /// `threshold_bytes`, else topology.
    pub fn from_size_classes(
        topo: &TopologyGraph,
        classes: &[SizeClassEntry],
        fallback: &str,
    ) -> Self {
        let (l1, l2, l3) = topology_thresholds(topo);
        let pick = |name: &str| {
            classes
                .iter()
                .find(|c| c.class.eq_ignore_ascii_case(name))
                .map(|c| c.winner.as_str())
                .unwrap_or(fallback)
        };
        let l1_thr = classes
            .iter()
            .find(|c| c.class.eq_ignore_ascii_case("L1"))
            .map(|c| c.threshold_bytes)
            .unwrap_or(l1);
        let l2_thr = classes
            .iter()
            .find(|c| c.class.eq_ignore_ascii_case("L2"))
            .map(|c| c.threshold_bytes)
            .unwrap_or(l2);
        let l3_thr = classes
            .iter()
            .find(|c| c.class.eq_ignore_ascii_case("L3"))
            .map(|c| c.threshold_bytes)
            .unwrap_or(l3);
        Self::from_thresholds(
            l1_thr,
            l2_thr,
            l3_thr,
            pick("L1"),
            pick("L2"),
            pick("L3"),
            pick("DRAM"),
            fallback,
        )
    }

    /// Construct from explicit thresholds and variant ids.
    pub fn from_thresholds(
        l1: u64,
        l2: u64,
        l3: u64,
        l1_variant: &str,
        l2_variant: &str,
        l3_variant: &str,
        dram_variant: &str,
        fallback: &str,
    ) -> Self {
        let root = DecisionNode::SizeBranch {
            threshold_bytes: l1,
            label: "L1D".into(),
            less: Box::new(DecisionNode::Select {
                variant: l1_variant.into(),
            }),
            greater_or_equal: Box::new(DecisionNode::SizeBranch {
                threshold_bytes: l2,
                label: "L2".into(),
                less: Box::new(DecisionNode::Select {
                    variant: l2_variant.into(),
                }),
                greater_or_equal: Box::new(DecisionNode::SizeBranch {
                    threshold_bytes: l3,
                    label: "L3".into(),
                    less: Box::new(DecisionNode::Select {
                        variant: l3_variant.into(),
                    }),
                    greater_or_equal: Box::new(DecisionNode::Select {
                        variant: dram_variant.into(),
                    }),
                }),
            }),
        };

        Self {
            l1_bytes: l1,
            l2_bytes: l2,
            l3_bytes: l3,
            root,
            fallback_variant: fallback.into(),
        }
    }

    /// Evaluate tree for a working-set size in bytes.
    pub fn evaluate(&self, size_bytes: u64) -> &str {
        let mut node = &self.root;
        loop {
            match node {
                DecisionNode::Select { variant } => return variant.as_str(),
                DecisionNode::SizeBranch {
                    threshold_bytes,
                    less,
                    greater_or_equal,
                    ..
                } => {
                    node = if size_bytes < *threshold_bytes {
                        less
                    } else {
                        greater_or_equal
                    };
                }
            }
        }
    }

    /// Explain path taken for a size.
    pub fn explain(&self, size_bytes: u64) -> Vec<String> {
        let mut steps = Vec::new();
        let mut node = &self.root;
        loop {
            match node {
                DecisionNode::Select { variant } => {
                    steps.push(format!("select variant '{variant}'"));
                    break;
                }
                DecisionNode::SizeBranch {
                    threshold_bytes,
                    label,
                    less,
                    greater_or_equal,
                } => {
                    if size_bytes < *threshold_bytes {
                        steps.push(format!(
                            "size {size_bytes} < {label} threshold {threshold_bytes} → less"
                        ));
                        node = less;
                    } else {
                        steps.push(format!(
                            "size {size_bytes} ≥ {label} threshold {threshold_bytes} → greater_or_equal"
                        ));
                        node = greater_or_equal;
                    }
                }
            }
        }
        steps
    }
}

/// Topology-derived L1/L2/L3 thresholds (bytes).
pub fn topology_thresholds(topo: &TopologyGraph) -> (u64, u64, u64) {
    let l1 = topo.typical_l1d_bytes().max(32 * 1024);
    let l2 = topo.typical_l2_bytes().max(512 * 1024);
    let l3 = topo
        .packages
        .first()
        .and_then(|p| p.domains.first())
        .and_then(|d| d.l3_bytes())
        .unwrap_or(32 * 1024 * 1024);
    (l1, l2, l3)
}

/// Plan specialization given topology.
pub fn plan_size_specialization(topo: &TopologyGraph, cfg: &SpecializeConfig) -> SpecializeResult {
    let mut notes = Vec::new();
    if !cfg.size_dependent {
        notes.push("size-dependent specialization disabled; single fallback variant".into());
        return SpecializeResult {
            tree: DecisionTree {
                l1_bytes: 0,
                l2_bytes: 0,
                l3_bytes: 0,
                root: DecisionNode::Select {
                    variant: "baseline".into(),
                },
                fallback_variant: "baseline".into(),
            },
            notes,
        };
    }
    let tree = DecisionTree::from_topology(
        topo,
        "l1_resident",
        "l2_resident",
        "l3_resident",
        "dram_friendly",
        "baseline",
    );
    notes.push(format!(
        "thresholds L1D={} L2={} L3={}",
        tree.l1_bytes, tree.l2_bytes, tree.l3_bytes
    ));
    SpecializeResult { tree, notes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{CacheLevel, CacheNode, ComputeDomain, CoreNode, Package, ThreadNode};

    fn topo() -> TopologyGraph {
        TopologyGraph {
            packages: vec![Package {
                id: 0,
                domains: vec![ComputeDomain {
                    id: 0,
                    cores: vec![CoreNode {
                        id: 0,
                        threads: vec![ThreadNode { id: 0, apic_id: 0 }],
                        l1i_bytes: 32 * 1024,
                        l1d_bytes: 32 * 1024,
                        l2_bytes: 1024 * 1024,
                    }],
                    shared_caches: vec![CacheNode {
                        level: CacheLevel::L3,
                        size_bytes: 32 * 1024 * 1024,
                        shared_by_cores: vec![0],
                    }],
                }],
            }],
        }
    }

    #[test]
    fn size_dispatch() {
        let tree = DecisionTree::from_topology(&topo(), "a", "b", "c", "d", "baseline");
        assert_eq!(tree.evaluate(1024), "a");
        assert_eq!(tree.evaluate(100_000), "b");
        assert_eq!(tree.evaluate(2_000_000), "c");
        assert_eq!(tree.evaluate(64_000_000), "d");
    }
}
