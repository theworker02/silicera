//! Mock hardware backends for CI and deterministic tests.

use crate::fingerprint::Fingerprint;
use crate::hardware::{
    EnvironmentSnapshot, HardwareBackend, HardwareInfo, SupportStatus, ValidationReportDto,
};
use crate::knowledge::{KnowledgePack, Microarch};
use crate::topology::{
    CacheLevel, CacheNode, ComputeDomain, CoreNode, Package, ThreadNode, TopologyGraph,
};
use crate::Result;

/// Configurable mock hardware.
#[derive(Debug, Clone)]
pub struct MockHardware {
    kind: MockKind,
}

#[derive(Debug, Clone)]
enum MockKind {
    Zen4SingleCcd,
    Zen5DualCcd,
    UnsupportedIntel,
    UnsupportedOldAmd,
}

impl MockHardware {
    /// Zen 4 single-CCD desktop-like mock (8c/16t, 32 MiB L3).
    pub fn zen4_single_ccd() -> Self {
        Self {
            kind: MockKind::Zen4SingleCcd,
        }
    }

    /// Zen 5 dual-CCD desktop-like mock (16c/32t, 2×32 MiB L3).
    pub fn zen5_dual_ccd() -> Self {
        Self {
            kind: MockKind::Zen5DualCcd,
        }
    }

    /// Non-AMD vendor — must surface graceful unsupported status.
    pub fn unsupported_intel() -> Self {
        Self {
            kind: MockKind::UnsupportedIntel,
        }
    }

    /// AMD vendor but outside Zen3/Zen4/Zen5 microarch support.
    pub fn unsupported_old_amd() -> Self {
        Self {
            kind: MockKind::UnsupportedOldAmd,
        }
    }
}

impl HardwareBackend for MockHardware {
    fn discover(&self, knowledge: &KnowledgePack) -> Result<HardwareInfo> {
        match &self.kind {
            MockKind::UnsupportedIntel => Ok(HardwareInfo {
                brand: "Mock GenuineIntel CPU".into(),
                vendor: "GenuineIntel".into(),
                family: 6,
                model: 0x9A,
                stepping: 0,
                support: SupportStatus::Unsupported {
                    reason: "vendor is GenuineIntel; Silicera targets AMD Zen3/Zen4/Zen5 only"
                        .into(),
                },
                topology: TopologyGraph::new(),
                fingerprint: None,
                validation: None,
                is_mock: true,
                environment: EnvironmentSnapshot::capture(),
            }),
            MockKind::UnsupportedOldAmd => Ok(HardwareInfo {
                brand: "Mock AMD Zen2".into(),
                vendor: "AuthenticAMD".into(),
                family: 0x17,
                model: 0x71,
                stepping: 0,
                support: SupportStatus::UnsupportedMicroarch {
                    reason: "family 17h (Zen 2) is outside supported set (Zen3/Zen4/Zen5)".into(),
                    family: Some(0x17),
                    model: Some(0x71),
                },
                topology: TopologyGraph::new(),
                fingerprint: None,
                validation: None,
                is_mock: true,
                environment: EnvironmentSnapshot::capture(),
            }),
            MockKind::Zen4SingleCcd => {
                build_supported(knowledge, Microarch::Zen4, 0x19, 0x61, 0, "Mock AMD Ryzen Zen4", 1, 8)
            }
            MockKind::Zen5DualCcd => {
                build_supported(knowledge, Microarch::Zen5, 0x1A, 0x44, 0, "Mock AMD Ryzen Zen5", 2, 8)
            }
        }
    }
}

fn build_supported(
    knowledge: &KnowledgePack,
    microarch: Microarch,
    family: u32,
    model: u32,
    stepping: u32,
    brand: &str,
    ccd_count: u32,
    cores_per_ccd: u32,
) -> Result<HardwareInfo> {
    let pack = knowledge
        .get(microarch)
        .expect("builtin pack must exist");
    let topology = synthesize_topology(pack, ccd_count, cores_per_ccd);
    let report = pack.validate_topology(&topology);
    let fingerprint = Fingerprint::from_topology(microarch, family, model, stepping, &topology);

    Ok(HardwareInfo {
        brand: brand.into(),
        vendor: "AuthenticAMD".into(),
        family,
        model,
        stepping,
        support: SupportStatus::Supported { microarch },
        topology,
        fingerprint: Some(fingerprint),
        validation: Some(ValidationReportDto::from(report)),
        is_mock: true,
        environment: EnvironmentSnapshot::capture(),
    })
}

fn synthesize_topology(
    pack: &crate::knowledge::ArchitecturePack,
    ccd_count: u32,
    cores_per_ccd: u32,
) -> TopologyGraph {
    let mut domains = Vec::new();
    for d in 0..ccd_count {
        let mut cores = Vec::new();
        let mut shared_core_ids = Vec::new();
        for c in 0..cores_per_ccd {
            let id = d * cores_per_ccd + c;
            shared_core_ids.push(id);
            cores.push(CoreNode {
                id,
                threads: vec![
                    ThreadNode {
                        id: 0,
                        apic_id: id * 2,
                    },
                    ThreadNode {
                        id: 1,
                        apic_id: id * 2 + 1,
                    },
                ],
                l1i_bytes: pack.caches.l1i_bytes,
                l1d_bytes: pack.caches.l1d_bytes,
                l2_bytes: pack.caches.l2_bytes,
            });
        }
        let l3 = pack.caches.l3_per_ccd_max;
        domains.push(ComputeDomain {
            id: d,
            cores,
            shared_caches: vec![CacheNode {
                level: CacheLevel::L3,
                size_bytes: l3,
                shared_by_cores: shared_core_ids,
            }],
        });
    }
    TopologyGraph {
        packages: vec![Package { id: 0, domains }],
    }
}
