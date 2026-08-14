//! Machine fingerprinting.
//!
//! Fingerprints identify a **class of machine configuration** suitable for
//! profile matching. They are **not authentication**, **not serial numbers**,
//! and **not device-unique secrets**.
//!
//! Format: `SLC:AMD:ZENn:<family>:<model>:<stepping>:<topo_hash>:<cache_hash>`
//!
//! See `docs/security/fingerprint.md` and `docs/concepts/hardware-identity.md`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::knowledge::Microarch;
use crate::topology::TopologyGraph;
use crate::Result;

/// Parsed components of a Silicera fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintComponents {
    /// Vendor tag (always `AMD` for supported hosts).
    pub vendor: String,
    /// Microarchitecture tag (`ZEN3`, `ZEN4`, `ZEN5`).
    pub microarch: String,
    /// CPUID family.
    pub family: u32,
    /// CPUID model.
    pub model: u32,
    /// CPUID stepping.
    pub stepping: u32,
    /// Truncated topology hash (16 hex chars).
    pub topo_hash: String,
    /// Truncated cache-geometry hash (16 hex chars).
    pub cache_hash: String,
}

/// Full fingerprint string plus structured components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    /// Canonical string form.
    pub value: String,
    /// Structured fields.
    pub components: FingerprintComponents,
}

impl Fingerprint {
    /// Build a fingerprint from microarch + topology.
    pub fn from_topology(microarch: Microarch, family: u32, model: u32, stepping: u32, topo: &TopologyGraph) -> Self {
        let topo_bytes = topo.fingerprint_bytes();
        let cache_bytes = topo.cache_fingerprint_bytes();
        let topo_hash = truncate_hash(&topo_bytes);
        let cache_hash = truncate_hash(&cache_bytes);
        let components = FingerprintComponents {
            vendor: "AMD".to_string(),
            microarch: microarch.tag().to_string(),
            family,
            model,
            stepping,
            topo_hash: topo_hash.clone(),
            cache_hash: cache_hash.clone(),
        };
        let value = format!(
            "SLC:AMD:{}:{:02X}:{:02X}:{:02X}:{}:{}",
            microarch.tag(),
            family,
            model,
            stepping,
            topo_hash,
            cache_hash
        );
        Self { value, components }
    }

    /// Parse a fingerprint string.
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 8 || parts[0] != "SLC" {
            return Err(crate::SiliceraError::Parse(format!(
                "invalid fingerprint (expected SLC:AMD:ZENn:...): {s}"
            )));
        }
        let components = FingerprintComponents {
            vendor: parts[1].to_string(),
            microarch: parts[2].to_string(),
            family: u32::from_str_radix(parts[3], 16).map_err(|e| {
                crate::SiliceraError::Parse(format!("family: {e}"))
            })?,
            model: u32::from_str_radix(parts[4], 16).map_err(|e| {
                crate::SiliceraError::Parse(format!("model: {e}"))
            })?,
            stepping: u32::from_str_radix(parts[5], 16).map_err(|e| {
                crate::SiliceraError::Parse(format!("stepping: {e}"))
            })?,
            topo_hash: parts[6].to_string(),
            cache_hash: parts[7].to_string(),
        };
        Ok(Self {
            value: s.to_string(),
            components,
        })
    }

    /// True if vendor/microarch/family/model match (topology/cache may differ).
    pub fn same_silicon_class(&self, other: &Fingerprint) -> bool {
        self.components.vendor == other.components.vendor
            && self.components.microarch == other.components.microarch
            && self.components.family == other.components.family
            && self.components.model == other.components.model
    }

    /// Exact match including topology and cache hashes.
    pub fn exact_match(&self, other: &Fingerprint) -> bool {
        self.value == other.value
    }
}

impl std::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

fn truncate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let full = hasher.finalize();
    hex::encode(&full[..8])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{CacheLevel, CacheNode, ComputeDomain, CoreNode, Package, ThreadNode, TopologyGraph};

    fn sample_topo() -> TopologyGraph {
        let mut g = TopologyGraph::new();
        let mut pkg = Package { id: 0, domains: vec![] };
        let mut ccd = ComputeDomain { id: 0, cores: vec![], shared_caches: vec![] };
        ccd.shared_caches.push(CacheNode {
            level: CacheLevel::L3,
            size_bytes: 32 * 1024 * 1024,
            shared_by_cores: vec![0, 1],
        });
        ccd.cores.push(CoreNode {
            id: 0,
            threads: vec![ThreadNode { id: 0, apic_id: 0 }, ThreadNode { id: 1, apic_id: 1 }],
            l1i_bytes: 32 * 1024,
            l1d_bytes: 32 * 1024,
            l2_bytes: 1024 * 1024,
        });
        pkg.domains.push(ccd);
        g.packages.push(pkg);
        g
    }

    #[test]
    fn roundtrip_fingerprint() {
        let fp = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &sample_topo());
        let parsed = Fingerprint::parse(&fp.value).unwrap();
        assert_eq!(fp, parsed);
        assert!(fp.value.starts_with("SLC:AMD:ZEN5:"));
    }
}
