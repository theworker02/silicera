//! Hardware topology graph: Package → Compute Domain → Core → Thread + Shared Cache.
//!
//! Models AMD Chiplet / CCD structure at a level useful for specialization decisions.
//! This is an abstract research model, not a substitute for AMD's internal topology docs.

use serde::{Deserialize, Serialize};

/// Cache hierarchy level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheLevel {
    /// Instruction L1.
    L1I,
    /// Data L1.
    L1D,
    /// Unified L2.
    L2,
    /// Shared L3 (typically per CCD on Zen).
    L3,
}

impl CacheLevel {
    /// Short label.
    pub fn label(self) -> &'static str {
        match self {
            CacheLevel::L1I => "L1I",
            CacheLevel::L1D => "L1D",
            CacheLevel::L2 => "L2",
            CacheLevel::L3 => "L3",
        }
    }
}

/// A shared cache node (typically L3 per compute domain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheNode {
    /// Cache level.
    pub level: CacheLevel,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Core IDs that share this cache.
    pub shared_by_cores: Vec<u32>,
}

/// Logical SMT thread / APIC binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadNode {
    /// Logical thread index within the core.
    pub id: u32,
    /// APIC ID when known (0 if mock / unknown).
    pub apic_id: u32,
}

/// Physical core with private caches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreNode {
    /// Core ID within the compute domain.
    pub id: u32,
    /// SMT threads.
    pub threads: Vec<ThreadNode>,
    /// L1I size (bytes).
    pub l1i_bytes: u64,
    /// L1D size (bytes).
    pub l1d_bytes: u64,
    /// L2 size (bytes).
    pub l2_bytes: u64,
}

/// Compute domain — maps to an AMD CCD (Core Complex Die) when known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeDomain {
    /// Domain ID within the package.
    pub id: u32,
    /// Cores in this domain.
    pub cores: Vec<CoreNode>,
    /// Shared caches (typically L3).
    pub shared_caches: Vec<CacheNode>,
}

impl ComputeDomain {
    /// Total logical threads in this domain.
    pub fn thread_count(&self) -> usize {
        self.cores.iter().map(|c| c.threads.len()).sum()
    }

    /// L3 size if present.
    pub fn l3_bytes(&self) -> Option<u64> {
        self.shared_caches
            .iter()
            .find(|c| c.level == CacheLevel::L3)
            .map(|c| c.size_bytes)
    }
}

/// CPU package / socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// Package ID.
    pub id: u32,
    /// Compute domains (CCDs).
    pub domains: Vec<ComputeDomain>,
}

impl Package {
    /// Total physical cores.
    pub fn core_count(&self) -> usize {
        self.domains.iter().map(|d| d.cores.len()).sum()
    }

    /// Total logical threads.
    pub fn thread_count(&self) -> usize {
        self.domains.iter().map(|d| d.thread_count()).sum()
    }
}

/// Full machine topology graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TopologyGraph {
    /// Packages present.
    pub packages: Vec<Package>,
}

impl TopologyGraph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total physical cores across packages.
    pub fn core_count(&self) -> usize {
        self.packages.iter().map(|p| p.core_count()).sum()
    }

    /// Total logical threads.
    pub fn thread_count(&self) -> usize {
        self.packages.iter().map(|p| p.thread_count()).sum()
    }

    /// Number of compute domains (CCDs).
    pub fn domain_count(&self) -> usize {
        self.packages.iter().map(|p| p.domains.len()).sum()
    }

    /// Aggregate L3 across all domains.
    pub fn total_l3_bytes(&self) -> u64 {
        self.packages
            .iter()
            .flat_map(|p| &p.domains)
            .filter_map(|d| d.l3_bytes())
            .sum()
    }

    /// Representative per-core L2 size (first core), or 0.
    pub fn typical_l2_bytes(&self) -> u64 {
        self.packages
            .first()
            .and_then(|p| p.domains.first())
            .and_then(|d| d.cores.first())
            .map(|c| c.l2_bytes)
            .unwrap_or(0)
    }

    /// Representative L1D size.
    pub fn typical_l1d_bytes(&self) -> u64 {
        self.packages
            .first()
            .and_then(|p| p.domains.first())
            .and_then(|d| d.cores.first())
            .map(|c| c.l1d_bytes)
            .unwrap_or(0)
    }

    /// Bytes hashed into the topology portion of the fingerprint.
    pub fn fingerprint_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.packages.len() as u32).to_le_bytes());
        for pkg in &self.packages {
            out.extend_from_slice(&pkg.id.to_le_bytes());
            out.extend_from_slice(&(pkg.domains.len() as u32).to_le_bytes());
            for dom in &pkg.domains {
                out.extend_from_slice(&dom.id.to_le_bytes());
                out.extend_from_slice(&(dom.cores.len() as u32).to_le_bytes());
                for core in &dom.cores {
                    out.extend_from_slice(&core.id.to_le_bytes());
                    out.extend_from_slice(&(core.threads.len() as u32).to_le_bytes());
                }
            }
        }
        out
    }

    /// Bytes hashed into the cache-geometry portion of the fingerprint.
    pub fn cache_fingerprint_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for pkg in &self.packages {
            for dom in &pkg.domains {
                if let Some(l3) = dom.l3_bytes() {
                    out.extend_from_slice(&l3.to_le_bytes());
                }
                for core in &dom.cores {
                    out.extend_from_slice(&core.l1i_bytes.to_le_bytes());
                    out.extend_from_slice(&core.l1d_bytes.to_le_bytes());
                    out.extend_from_slice(&core.l2_bytes.to_le_bytes());
                }
            }
        }
        out
    }

    /// Human-readable summary lines for CLI.
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "packages={}  domains(CCDs)={}  cores={}  threads={}  L3_total={}",
            self.packages.len(),
            self.domain_count(),
            self.core_count(),
            self.thread_count(),
            format_bytes(self.total_l3_bytes())
        ));
        for pkg in &self.packages {
            for dom in &pkg.domains {
                lines.push(format!(
                    "  package {} domain {} : cores={} threads={} L3={}",
                    pkg.id,
                    dom.id,
                    dom.cores.len(),
                    dom.thread_count(),
                    dom.l3_bytes()
                        .map(format_bytes)
                        .unwrap_or_else(|| "n/a".into())
                ));
            }
        }
        lines
    }
}

/// Format byte size for display.
pub fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.2} GiB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.2} MiB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.2} KiB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_counts() {
        let g = TopologyGraph::new();
        assert_eq!(g.core_count(), 0);
        assert_eq!(g.total_l3_bytes(), 0);
    }
}
