//! # Silicera
//!
//! Experimental research library for **hardware-native program specialization** on
//! modern AMD Zen microarchitectures.
//!
//! Silicera treats the physical machine — topology, cache geometry, and measured
//! behavior — as an input to specialization. It is **not** affiliated with, endorsed
//! by, or certified by Advanced Micro Devices, Inc.
//!
//! ## This release
//!
//! - Discovery, tournaments, HNEP v2, runtime fallback for AMD Zen3 / Zen4 / Zen5
//! - Compiler feedback export (SCF), host-ISA harness arms, alignment/placement research
//! - Architecture compare scaffolding, profile lifecycle health, and opt-in fleet-share schema
//!
//! ## Non-goals
//!
//! - Kernel drivers, BIOS/firmware modification, or overclocking
//! - LLM-in-the-loop optimization
//! - Fabricated speedups or vendor partnership claims
//! - Automatic cloud upload of profiles
//!
//! ## Crate map
//!
//! | Crate | Role |
//! |-------|------|
//! | `silicera` | Discovery, measurement, HNEP, specialization (this crate) |
//! | `silicera-runtime` | Profile load + cheap dispatch (no lab dependency) |
//! | `silicera-lab` | Microbenchmarks, tournaments, terminal lab UI |
//! | `silicera-cli` | `silicera` binary |

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod arch_compare;
pub mod archive;
pub mod brand;
pub mod compare;
pub mod counters;
pub mod error;
pub mod feedback;
pub mod fingerprint;
pub mod fleet;
pub mod hardware;
pub mod hnep;
pub mod knowledge;
pub mod lifecycle;
pub mod macros;
pub mod measure;
pub mod profile_diff;
pub mod remarks;
pub mod repro;
pub mod retrain;
pub mod specialize;
pub mod staleness;
pub mod topology;
pub mod tournament;
pub mod tpm;
pub mod variant;

pub use arch_compare::{
    compare_architectures, compare_mock_zen4_zen5, parse_microarch_tag, side_from_info, ArchCompareReport,
    ArchSide,
};
pub use archive::{
    entry_from_eval_json, ArchiveEntry, ResultsArchive, ARCHIVE_FORMAT, ARCHIVE_VERSION,
};
pub use brand::{
    brand_json, BrandInfo, AFFILIATION_DISCLAIMER, BRAND_LINE, FUNDING_URL, GITHUB_SPONSOR,
    HOMEPAGE, LICENSE, LOGO_RELATIVE, NAME, PHASE, RELEASE_LINE, REPOSITORY, TAGLINE, THANKS_DEV,
    VERSION,
};
pub use compare::{
    compare_profiles, compare_with_placeholder, CompareReport, DivergenceKind, MachineBPlaceholder,
    MeasurementEcho, SanitizedSplitExport, SplitVerdict, StrategyVector, SILICON_SPLIT_PROTOCOL,
    SILICON_SPLIT_PROTOCOL_VERSION,
};
pub use counters::{probe_counters, CounterAvailability, CounterProbe};
pub use error::{Result, SiliceraError};
pub use feedback::{CompilerFeedback, ScfSizeClass, ScfWorkload, SCF_FORMAT, SCF_VERSION};
pub use fingerprint::{Fingerprint, FingerprintComponents};
pub use fleet::{FleetShare, FleetWorkloadShare, FLEET_FORMAT, FLEET_VERSION};
pub use hardware::{
    detect_hardware, EnvironmentSnapshot, HardwareBackend, HardwareInfo, MockHardware,
    SupportStatus,
};
pub use hnep::{
    Confidence, HnepHeader, HnepProfile, IntegrityDigest, SizeClassEntry, WorkloadEntry,
    HNEP_FORMAT, HNEP_VERSION, HNEP_VERSION_MAX_SUPPORTED, HNEP_VERSION_MIN_SUPPORTED,
};
pub use knowledge::{ArchitecturePack, KnowledgePack, Microarch};
pub use lifecycle::{
    assess_profile_staleness, plan_profile_retrain, HealthGrade, ProfileHealth,
};
pub use measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary, Stability};
pub use remarks::{Remark, RemarkArg, RemarksBundle, REMARKS_FORMAT, REMARKS_VERSION};
pub use repro::{ReproPack, REPRO_FORMAT, REPRO_VERSION};
pub use retrain::{
    parse_only_filter, plan_partial_retrain, target_dependencies, target_matches_filter,
    PartialRetrainPlan, RetrainDep, TargetDeps,
};
pub use specialize::{DecisionTree, SpecializeConfig, SpecializeResult};
pub use staleness::{
    assess_staleness, DriftSeverity, DriftSignal, RetrainRecommendation, StalenessPolicy,
    StalenessReport,
};
pub use topology::{CacheLevel, CacheNode, ComputeDomain, CoreNode, Package, ThreadNode, TopologyGraph};
pub use tournament::{Tournament, TournamentConfig, TournamentResult, RoundOutcome};
pub use variant::{CorrectnessCheck, Variant, VariantId, VariantSet, VariantSetMeta};
