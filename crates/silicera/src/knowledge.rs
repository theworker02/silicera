//! AMD architecture knowledge packs.
//!
//! Knowledge packs encode **public / measured** microarchitecture facts used to
//! validate discovery and guide specialization decisions. They are distinct from
//! HNEP profiles (which are measured on a specific machine).
//!
//! Sources are limited to publicly documented AMD materials, CPUID leaves, and
//! reproducible measurements. No confidential AMD IP is claimed or required.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::topology::TopologyGraph;

/// Supported AMD Zen microarchitectures (Zen3/Zen4/Zen5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Microarch {
    /// Zen 3 (e.g. Vermeer, Cezanne).
    Zen3,
    /// Zen 4 (e.g. Raphael, Storm Peak).
    Zen4,
    /// Zen 5 (e.g. Granite Ridge, Strix Point).
    Zen5,
}

impl Microarch {
    /// Fingerprint tag.
    pub fn tag(self) -> &'static str {
        match self {
            Microarch::Zen3 => "ZEN3",
            Microarch::Zen4 => "ZEN4",
            Microarch::Zen5 => "ZEN5",
        }
    }

    /// Parse from tag string.
    pub fn from_tag(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "ZEN3" | "ZEN_3" => Some(Microarch::Zen3),
            "ZEN4" | "ZEN_4" => Some(Microarch::Zen4),
            "ZEN5" | "ZEN_5" => Some(Microarch::Zen5),
            _ => None,
        }
    }

    /// All supported microarchitectures (Zen3/Zen4/Zen5).
    pub fn all() -> &'static [Microarch] {
        &[Microarch::Zen3, Microarch::Zen4, Microarch::Zen5]
    }
}

impl std::fmt::Display for Microarch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag())
    }
}

/// Expected cache geometry ranges (public knowledge / measured).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheExpectations {
    /// Typical L1D per core (bytes).
    pub l1d_bytes: u64,
    /// Typical L1I per core (bytes).
    pub l1i_bytes: u64,
    /// Typical L2 per core (bytes).
    pub l2_bytes: u64,
    /// Typical L3 per CCD lower bound (bytes).
    pub l3_per_ccd_min: u64,
    /// Typical L3 per CCD upper bound (bytes).
    pub l3_per_ccd_max: u64,
}

/// Architecture knowledge pack for one Zen generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturePack {
    /// Microarchitecture.
    pub microarch: Microarch,
    /// Human-readable name.
    pub name: String,
    /// Short notes (public knowledge only).
    pub notes: String,
    /// CPUID family values commonly associated (hex in docs; stored as u32).
    pub families: Vec<u32>,
    /// Model ranges as (min, max) inclusive, interpreted within family.
    pub model_ranges: Vec<(u32, u32)>,
    /// Cache expectations.
    pub caches: CacheExpectations,
    /// Whether SMT (2 threads/core) is typical.
    pub smt_typical: bool,
    /// Knowledge pack schema version.
    pub pack_version: u32,
    /// Provenance / source notes.
    pub provenance: String,
}

impl ArchitecturePack {
    /// Validate observed topology against this pack's expectations.
    ///
    /// Returns warnings (soft) rather than hard failures for size variance across SKUs.
    pub fn validate_topology(&self, topo: &TopologyGraph) -> ValidationReport {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if topo.core_count() == 0 {
            errors.push("topology has zero cores".into());
        }

        for pkg in &topo.packages {
            for dom in &pkg.domains {
                if let Some(l3) = dom.l3_bytes() {
                    if l3 < self.caches.l3_per_ccd_min || l3 > self.caches.l3_per_ccd_max {
                        warnings.push(format!(
                            "domain {} L3={} outside expected range [{}, {}] for {}",
                            dom.id,
                            l3,
                            self.caches.l3_per_ccd_min,
                            self.caches.l3_per_ccd_max,
                            self.microarch
                        ));
                    }
                } else {
                    warnings.push(format!("domain {} has no L3 node", dom.id));
                }
                for core in &dom.cores {
                    if core.l1d_bytes != self.caches.l1d_bytes {
                        warnings.push(format!(
                            "core {} L1D={} != expected {}",
                            core.id, core.l1d_bytes, self.caches.l1d_bytes
                        ));
                    }
                    if core.l2_bytes != self.caches.l2_bytes {
                        warnings.push(format!(
                            "core {} L2={} != expected {} (SKU variance possible)",
                            core.id, core.l2_bytes, self.caches.l2_bytes
                        ));
                    }
                }
            }
        }

        ValidationReport { warnings, errors }
    }

    /// True if family/model could belong to this pack.
    pub fn matches_cpuid(&self, family: u32, model: u32) -> bool {
        if !self.families.contains(&family) {
            return false;
        }
        if self.model_ranges.is_empty() {
            return true;
        }
        self.model_ranges
            .iter()
            .any(|(lo, hi)| model >= *lo && model <= *hi)
    }
}

/// Soft/hard validation outcomes from knowledge-pack checks.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// Soft mismatches (SKU variance, incomplete leaves).
    pub warnings: Vec<String>,
    /// Hard failures.
    pub errors: Vec<String>,
}

impl ValidationReport {
    /// True if no hard errors.
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Collection of architecture packs loaded from `profiles/amd/*.toml`.
#[derive(Debug, Clone, Default)]
pub struct KnowledgePack {
    /// Packs by microarch.
    pub packs: Vec<ArchitecturePack>,
}

impl KnowledgePack {
    /// Built-in Zen3/Zen4/Zen5 packs (always available; TOML files may override/extend).
    pub fn builtin() -> Self {
        Self {
            packs: vec![zen3_pack(), zen4_pack(), zen5_pack()],
        }
    }

    /// Load packs from a directory of TOML files; falls back to builtin on missing dir.
    pub fn load_dir(dir: &Path) -> Result<Self> {
        if !dir.exists() {
            return Ok(Self::builtin());
        }
        let mut packs = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            let pack: ArchitecturePack = toml::from_str(&text)?;
            packs.push(pack);
        }
        if packs.is_empty() {
            return Ok(Self::builtin());
        }
        Ok(Self { packs })
    }

    /// Resolve microarch from family/model using pack tables.
    ///
    /// When multiple packs could match (family 19h spans Zen3/Zen4), prefer the
    /// most specific model-range hit; Zen5 (family 1Ah) is unambiguous.
    pub fn resolve_microarch(&self, family: u32, model: u32) -> Option<Microarch> {
        // Prefer later packs when both match so Zen4 ranges win over broad Zen3.
        self.packs
            .iter()
            .rev()
            .find(|p| p.matches_cpuid(family, model))
            .map(|p| p.microarch)
    }

    /// Get pack for microarch.
    pub fn get(&self, m: Microarch) -> Option<&ArchitecturePack> {
        self.packs.iter().find(|p| p.microarch == m)
    }
}

fn zen3_pack() -> ArchitecturePack {
    ArchitecturePack {
        microarch: Microarch::Zen3,
        name: "Zen 3".into(),
        notes: "Public knowledge: unified 8-core CCX, 32 KiB L1D/L1I, 512 KiB L2, up to 32 MiB L3 per CCD (desktop)."
            .into(),
        families: vec![0x19],
        // Vermeer/Cezanne/Milan-class public model bands (excludes Zen4 0x10–0x1F / 0x60+).
        model_ranges: vec![(0x00, 0x0F), (0x20, 0x5F)],
        caches: CacheExpectations {
            l1d_bytes: 32 * 1024,
            l1i_bytes: 32 * 1024,
            l2_bytes: 512 * 1024,
            l3_per_ccd_min: 8 * 1024 * 1024,
            l3_per_ccd_max: 32 * 1024 * 1024,
        },
        smt_typical: true,
        pack_version: 1,
        provenance: "Public AMD Zen 3 architecture materials + CPUID cache leaves; not AMD-confidential."
            .into(),
    }
}

fn zen4_pack() -> ArchitecturePack {
    ArchitecturePack {
        microarch: Microarch::Zen4,
        name: "Zen 4".into(),
        notes: "Public knowledge: up to 8 cores/CCD, 32 KiB L1D/L1I, 1 MiB L2, up to 32 MiB L3 per CCD."
            .into(),
        families: vec![0x19],
        model_ranges: vec![(0x60, 0x7F), (0x10, 0x1F)],
        caches: CacheExpectations {
            l1d_bytes: 32 * 1024,
            l1i_bytes: 32 * 1024,
            l2_bytes: 1024 * 1024,
            l3_per_ccd_min: 8 * 1024 * 1024,
            l3_per_ccd_max: 32 * 1024 * 1024,
        },
        smt_typical: true,
        pack_version: 1,
        provenance: "Public AMD Zen 4 architecture materials + CPUID cache leaves; not AMD-confidential."
            .into(),
    }
}

fn zen5_pack() -> ArchitecturePack {
    ArchitecturePack {
        microarch: Microarch::Zen5,
        name: "Zen 5".into(),
        notes: "Public knowledge: family 1Ah, dual-CCD desktop parts (e.g. 9950X), 48 KiB L1D / 32 KiB L1I, 1 MiB L2, up to 32 MiB L3 per CCD."
            .into(),
        families: vec![0x1A],
        model_ranges: vec![(0x00, 0xFF)],
        caches: CacheExpectations {
            l1d_bytes: 48 * 1024,
            l1i_bytes: 32 * 1024,
            l2_bytes: 1024 * 1024,
            l3_per_ccd_min: 8 * 1024 * 1024,
            l3_per_ccd_max: 32 * 1024 * 1024,
        },
        smt_typical: true,
        pack_version: 1,
        provenance: "Public AMD Zen 5 architecture materials + CPUID cache leaves; not AMD-confidential."
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zen5_family_resolves() {
        let kp = KnowledgePack::builtin();
        assert_eq!(kp.resolve_microarch(0x1A, 0x44), Some(Microarch::Zen5));
    }

    #[test]
    fn tags_roundtrip() {
        for m in Microarch::all() {
            assert_eq!(Microarch::from_tag(m.tag()), Some(*m));
        }
    }
}
