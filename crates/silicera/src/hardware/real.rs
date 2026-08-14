//! Real CPUID-based AMD hardware discovery.
//!
//! Uses public CPUID leaves only. No MSR access, no kernel driver, no firmware writes.

use raw_cpuid::{CpuId, CpuIdReaderNative};

use crate::fingerprint::Fingerprint;
use crate::hardware::{
    EnvironmentSnapshot, HardwareInfo, SupportStatus, ValidationReportDto,
};
use crate::knowledge::{KnowledgePack, Microarch};
use crate::topology::{
    CacheLevel, CacheNode, ComputeDomain, CoreNode, Package, ThreadNode, TopologyGraph,
};
use crate::Result;

type HostCpuId = CpuId<CpuIdReaderNative>;

/// Discover hardware via CPUID and knowledge-pack validation.
pub fn detect_real_hardware(knowledge: &KnowledgePack) -> Result<HardwareInfo> {
    let cpuid = CpuId::new();
    let env = EnvironmentSnapshot::capture();

    let vendor = match cpuid.get_vendor_info() {
        Some(v) => v.as_str().to_string(),
        None => {
            return Ok(unsupported(
                "unknown",
                0,
                0,
                0,
                "CPUID vendor leaf unavailable",
                env,
            ));
        }
    };

    if vendor != "AuthenticAMD" {
        return Ok(unsupported(
            &vendor,
            0,
            0,
            0,
            &format!(
                "vendor is {vendor}; Silicera targets AMD Zen3/Zen4/Zen5 (AuthenticAMD) only"
            ),
            env,
        ));
    }

    // raw-cpuid already returns effective AMD family/model (base+extended).
    let (family, model, stepping) = match cpuid.get_feature_info() {
        Some(fi) => (
            fi.family_id() as u32,
            fi.model_id() as u32,
            fi.stepping_id() as u32,
        ),
        None => {
            return Ok(unsupported(
                &vendor,
                0,
                0,
                0,
                "CPUID feature leaf unavailable",
                env,
            ));
        }
    };

    let brand = cpuid
        .get_processor_brand_string()
        .map(|b| b.as_str().trim().to_string())
        .unwrap_or_else(|| "AMD processor".into());

    let microarch = match knowledge.resolve_microarch(family, model) {
        Some(m) => m,
        None => {
            return Ok(HardwareInfo {
                brand,
                vendor,
                family,
                model,
                stepping,
                support: SupportStatus::UnsupportedMicroarch {
                    reason: format!(
                        "family={family:#x} model={model:#x} is not mapped to Zen3/Zen4/Zen5 in knowledge packs"
                    ),
                    family: Some(family),
                    model: Some(model),
                },
                topology: TopologyGraph::new(),
                fingerprint: None,
                validation: None,
                is_mock: false,
                environment: env,
            });
        }
    };

    let pack = knowledge.get(microarch).ok_or_else(|| {
        crate::SiliceraError::KnowledgePack(format!("missing pack for {microarch}"))
    })?;

    // Secondary validation: cache geometry must be plausible for the pack.
    // This is what makes AMD exclusivity "real" beyond a vendor string check.
    let topology = build_topology_from_cpuid(&cpuid, pack);
    let report = pack.validate_topology(&topology);

    if !report.ok() {
        return Ok(HardwareInfo {
            brand,
            vendor,
            family,
            model,
            stepping,
            support: SupportStatus::UnsupportedMicroarch {
                reason: format!(
                    "knowledge-pack validation failed for {microarch}: {}",
                    report.errors.join("; ")
                ),
                family: Some(family),
                model: Some(model),
            },
            topology,
            fingerprint: None,
            validation: Some(ValidationReportDto::from(report)),
            is_mock: false,
            environment: env,
        });
    }

    // Sanity: SMT / core counts should be coherent.
    if topology.core_count() == 0 {
        return Ok(HardwareInfo {
            brand,
            vendor,
            family,
            model,
            stepping,
            support: SupportStatus::UnsupportedMicroarch {
                reason: "unable to derive a non-empty topology from CPUID cache/topology leaves"
                    .into(),
                family: Some(family),
                model: Some(model),
            },
            topology,
            fingerprint: None,
            validation: Some(ValidationReportDto::from(report)),
            is_mock: false,
            environment: env,
        });
    }

    let fingerprint = Fingerprint::from_topology(microarch, family, model, stepping, &topology);

    Ok(HardwareInfo {
        brand,
        vendor,
        family,
        model,
        stepping,
        support: SupportStatus::Supported { microarch },
        topology,
        fingerprint: Some(fingerprint),
        validation: Some(ValidationReportDto::from(report)),
        is_mock: false,
        environment: env,
    })
}

fn unsupported(
    vendor: &str,
    family: u32,
    model: u32,
    stepping: u32,
    reason: &str,
    env: EnvironmentSnapshot,
) -> HardwareInfo {
    HardwareInfo {
        brand: String::new(),
        vendor: vendor.into(),
        family,
        model,
        stepping,
        support: SupportStatus::Unsupported {
            reason: reason.into(),
        },
        topology: TopologyGraph::new(),
        fingerprint: None,
        validation: None,
        is_mock: false,
        environment: env,
    }
}

fn build_topology_from_cpuid(
    cpuid: &HostCpuId,
    pack: &crate::knowledge::ArchitecturePack,
) -> TopologyGraph {
    // Prefer deterministic cache parameters from CPUID; fall back to pack defaults.
    let (l1d, l1i, l2, l3) = read_cache_sizes(cpuid, pack);
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as u32;
    let threads_per_core = if pack.smt_typical { 2u32 } else { 1u32 };
    let total_cores = (logical / threads_per_core).max(1);

    // Estimate CCD count from L3 / pack max (desktop dual-CCD heuristic).
    let l3_per = if l3 > 0 {
        // If total L3 looks like 2× pack max, dual CCD.
        if pack.caches.l3_per_ccd_max > 0 && l3 >= pack.caches.l3_per_ccd_max.saturating_mul(2) {
            pack.caches.l3_per_ccd_max
        } else if pack.caches.l3_per_ccd_max > 0 && l3 > pack.caches.l3_per_ccd_max {
            // Split evenly across estimated CCDs.
            let ccd_est = (l3 / pack.caches.l3_per_ccd_min).max(1);
            l3 / ccd_est
        } else {
            l3
        }
    } else {
        pack.caches.l3_per_ccd_max
    };

    let ccd_count = if l3_per > 0 && l3 >= l3_per.saturating_mul(2) {
        (l3 / l3_per).max(1).min(8) as u32
    } else if total_cores > 8 {
        // Dual-CCD heuristic for 12–16 core desktop parts.
        2
    } else {
        1
    };

    let cores_per_ccd = (total_cores / ccd_count).max(1);
    let mut domains = Vec::new();
    let mut core_id = 0u32;
    for d in 0..ccd_count {
        let mut cores = Vec::new();
        let mut shared = Vec::new();
        let n = if d == ccd_count - 1 {
            // Last CCD absorbs remainder.
            total_cores - core_id
        } else {
            cores_per_ccd
        };
        for _ in 0..n {
            shared.push(core_id);
            cores.push(CoreNode {
                id: core_id,
                threads: (0..threads_per_core)
                    .map(|t| ThreadNode {
                        id: t,
                        apic_id: core_id * threads_per_core + t,
                    })
                    .collect(),
                l1i_bytes: l1i,
                l1d_bytes: l1d,
                l2_bytes: l2,
            });
            core_id += 1;
        }
        domains.push(ComputeDomain {
            id: d,
            cores,
            shared_caches: vec![CacheNode {
                level: CacheLevel::L3,
                size_bytes: l3_per,
                shared_by_cores: shared,
            }],
        });
    }

    TopologyGraph {
        packages: vec![Package { id: 0, domains }],
    }
}

fn read_cache_sizes(
    cpuid: &HostCpuId,
    pack: &crate::knowledge::ArchitecturePack,
) -> (u64, u64, u64, u64) {
    let mut l1d = pack.caches.l1d_bytes;
    let mut l1i = pack.caches.l1i_bytes;
    let mut l2 = pack.caches.l2_bytes;
    let mut l3 = 0u64;

    // Deterministic cache parameters (leaf 4) — size = line * ways * sets.
    if let Some(cparams) = cpuid.get_cache_parameters() {
        for cache in cparams {
            let size = (cache.coherency_line_size() as u64)
                .saturating_mul(cache.associativity() as u64)
                .saturating_mul(cache.sets() as u64);
            if size == 0 {
                continue;
            }
            match (cache.level(), cache.cache_type()) {
                (1, raw_cpuid::CacheType::Data) => l1d = size,
                (1, raw_cpuid::CacheType::Instruction) => l1i = size,
                (2, _) => l2 = size,
                (3, _) => l3 = size,
                _ => {}
            }
        }
    }

    // AMD extended L2/L3 leaf (0x8000_0006). l3cache_size units: value * 512 KiB.
    if l3 == 0 {
        if let Some(l2l3) = cpuid.get_l2_l3_cache_and_tlb_info() {
            let raw = l2l3.l3cache_size() as u64;
            if raw > 0 {
                l3 = raw.saturating_mul(512).saturating_mul(1024);
            }
        }
    }

    if l3 == 0 {
        l3 = pack.caches.l3_per_ccd_max;
    }

    (l1d, l1i, l2, l3)
}

/// Helper used by doctests / diagnostics.
#[allow(dead_code)]
pub fn microarch_of(family: u32, model: u32) -> Option<Microarch> {
    KnowledgePack::builtin().resolve_microarch(family, model)
}
