//! Architecture-level comparison scaffolding (e.g. Zen4 vs Zen5).
//!
//! Compares **knowledge packs + topology fingerprints**, not invented
//! cross-SKU performance. Measured strategy divergence requires real HNEPs
//! from each microarchitecture class (see Silicon Split).

use serde::{Deserialize, Serialize};

use crate::hardware::HardwareInfo;
use crate::knowledge::{KnowledgePack, Microarch};
use crate::Result;

/// One side of an architecture comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchSide {
    /// Microarchitecture tag.
    pub microarch: String,
    /// Knowledge pack name.
    pub pack_name: String,
    /// Brand / mock brand.
    pub brand: String,
    /// Fingerprint when supported.
    pub fingerprint: Option<String>,
    /// Logical thread count.
    pub threads: usize,
    /// Physical core count (best-effort).
    pub cores: usize,
    /// Domain (CCD) count.
    pub domains: usize,
    /// Typical L1D bytes.
    pub l1d_bytes: u64,
    /// Typical L2 bytes.
    pub l2_bytes: u64,
    /// Aggregate L3 bytes across domains.
    pub l3_bytes: u64,
    /// Pack provenance note.
    pub provenance: String,
}

/// Diff between two architecture sides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchCompareReport {
    /// Side A.
    pub a: ArchSide,
    /// Side B.
    pub b: ArchSide,
    /// Structural differences (topology / pack expectations).
    pub differences: Vec<String>,
    /// Shared public capabilities / notes.
    pub notes: Vec<String>,
    /// Honest research status.
    pub research_status: String,
}

/// Build an [`ArchSide`] from discovered hardware + knowledge packs.
pub fn side_from_info(info: &HardwareInfo, packs: &KnowledgePack) -> Result<ArchSide> {
    let micro = match &info.support {
        crate::hardware::SupportStatus::Supported { microarch, .. } => *microarch,
        _ => {
            return Err(crate::SiliceraError::UnsupportedCpu(
                "architecture compare requires a supported Zen side".into(),
            ))
        }
    };
    let pack = packs
        .packs
        .iter()
        .find(|p| p.microarch == micro)
        .ok_or_else(|| {
            crate::SiliceraError::Parse(format!("no knowledge pack for {}", micro.tag()))
        })?;
    let l3: u64 = info
        .topology
        .packages
        .iter()
        .flat_map(|p| p.domains.iter())
        .filter_map(|d| d.l3_bytes())
        .sum();
    Ok(ArchSide {
        microarch: micro.tag().into(),
        pack_name: pack.name.clone(),
        brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        threads: info.topology.thread_count(),
        cores: info.topology.core_count(),
        domains: info.topology.domain_count(),
        l1d_bytes: info.topology.typical_l1d_bytes(),
        l2_bytes: info.topology.typical_l2_bytes(),
        l3_bytes: l3,
        provenance: pack.provenance.clone(),
    })
}

/// Compare two architecture sides (structural; not performance claims).
pub fn compare_architectures(a: ArchSide, b: ArchSide) -> ArchCompareReport {
    let mut differences = Vec::new();
    if a.microarch != b.microarch {
        differences.push(format!(
            "microarchitecture: {} vs {}",
            a.microarch, b.microarch
        ));
    }
    if a.domains != b.domains {
        differences.push(format!("compute domains (CCD): {} vs {}", a.domains, b.domains));
    }
    if a.cores != b.cores {
        differences.push(format!("cores: {} vs {}", a.cores, b.cores));
    }
    if a.threads != b.threads {
        differences.push(format!("threads: {} vs {}", a.threads, b.threads));
    }
    if a.l1d_bytes != b.l1d_bytes {
        differences.push(format!("typical L1D: {} vs {} bytes", a.l1d_bytes, b.l1d_bytes));
    }
    if a.l2_bytes != b.l2_bytes {
        differences.push(format!("typical L2: {} vs {} bytes", a.l2_bytes, b.l2_bytes));
    }
    if a.l3_bytes != b.l3_bytes {
        differences.push(format!("aggregate L3: {} vs {} bytes", a.l3_bytes, b.l3_bytes));
    }
    let notes = vec![
        "This report compares knowledge-pack expectations and discovered topology.".into(),
        "It does NOT claim which microarchitecture is faster.".into(),
        "Strategy divergence requires measured HNEPs from each class (Silicon Split).".into(),
    ];
    let research_status = if differences.is_empty() {
        "STRUCTURALLY SIMILAR — still measure; topology equality ≠ identical winners".into()
    } else {
        format!(
            "{} structural differences recorded — ready for dual-SKU measurement when hosts exist",
            differences.len()
        )
    };
    ArchCompareReport {
        a,
        b,
        differences,
        notes,
        research_status,
    }
}

/// Convenience: compare mock Zen4 vs Zen5 using builtin packs (CI-friendly).
pub fn compare_mock_zen4_zen5(packs: &KnowledgePack) -> Result<ArchCompareReport> {
    use crate::hardware::{HardwareBackend, MockHardware};
    let a = MockHardware::zen4_single_ccd().discover(packs)?;
    let b = MockHardware::zen5_dual_ccd().discover(packs)?;
    Ok(compare_architectures(
        side_from_info(&a, packs)?,
        side_from_info(&b, packs)?,
    ))
}

/// Parse microarch tag for CLI helpers.
pub fn parse_microarch_tag(s: &str) -> Option<Microarch> {
    match s.to_ascii_lowercase().as_str() {
        "zen3" => Some(Microarch::Zen3),
        "zen4" => Some(Microarch::Zen4),
        "zen5" => Some(Microarch::Zen5),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_zen4_vs_zen5_differs() {
        let packs = KnowledgePack::builtin();
        let report = compare_mock_zen4_zen5(&packs).unwrap();
        assert!(!report.differences.is_empty());
        assert_eq!(report.a.microarch, "ZEN4");
        assert_eq!(report.b.microarch, "ZEN5");
    }
}
