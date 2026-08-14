//! Hardware discovery backends.
//!
//! Real discovery uses CPUID (via `raw-cpuid`) and validates against AMD knowledge packs.
//! Mock backends enable CI on non-AMD hosts and deterministic tests.
//!
//! Unsupported CPUs return [`SupportStatus::Unsupported`] — never panic.

mod mock;
mod real;

pub use mock::MockHardware;
pub use real::detect_real_hardware;

use serde::{Deserialize, Serialize};

use crate::fingerprint::Fingerprint;
use crate::knowledge::{KnowledgePack, Microarch, ValidationReport};
use crate::topology::TopologyGraph;
use crate::{Result, SiliceraError};

/// Whether the host is usable for Silicera specialization (Zen3/Zen4/Zen5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportStatus {
    /// Fully supported AMD Zen microarchitecture.
    Supported {
        /// Resolved microarchitecture.
        microarch: Microarch,
    },
    /// AMD vendor but microarch outside the supported Zen3/Zen4/Zen5 set.
    UnsupportedMicroarch {
        /// Human-readable reason.
        reason: String,
        /// CPUID family if known.
        family: Option<u32>,
        /// CPUID model if known.
        model: Option<u32>,
    },
    /// Non-AMD or CPUID unavailable.
    Unsupported {
        /// Human-readable reason (printed by CLI; process exits gracefully).
        reason: String,
    },
}

impl SupportStatus {
    /// True if specialization may proceed.
    pub fn is_supported(&self) -> bool {
        matches!(self, SupportStatus::Supported { .. })
    }

    /// Exit-friendly message for operators.
    pub fn message(&self) -> String {
        match self {
            SupportStatus::Supported { microarch } => {
                format!("supported AMD microarchitecture: {microarch}")
            }
            SupportStatus::UnsupportedMicroarch { reason, .. } => {
                format!("unsupported AMD microarchitecture: {reason}")
            }
            SupportStatus::Unsupported { reason } => {
                format!("unsupported CPU: {reason}")
            }
        }
    }
}

/// Discovered hardware description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// Brand string from CPUID leaf 0x80000002–4 when available.
    pub brand: String,
    /// Vendor string (`AuthenticAMD`, mock tags, etc.).
    pub vendor: String,
    /// CPUID family.
    pub family: u32,
    /// CPUID model.
    pub model: u32,
    /// CPUID stepping.
    pub stepping: u32,
    /// Support classification.
    pub support: SupportStatus,
    /// Topology graph (empty if unsupported).
    pub topology: TopologyGraph,
    /// Machine fingerprint (present when supported).
    pub fingerprint: Option<Fingerprint>,
    /// Knowledge-pack validation (when supported).
    pub validation: Option<ValidationReportDto>,
    /// True when this info came from [`MockHardware`].
    pub is_mock: bool,
    /// Environment snapshot at discovery time.
    pub environment: EnvironmentSnapshot,
}

/// Serializable validation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReportDto {
    /// Soft warnings.
    pub warnings: Vec<String>,
    /// Hard errors.
    pub errors: Vec<String>,
}

impl From<ValidationReport> for ValidationReportDto {
    fn from(r: ValidationReport) -> Self {
        Self {
            warnings: r.warnings,
            errors: r.errors,
        }
    }
}

/// Captured environment for staleness detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    /// ISO-8601 UTC timestamp.
    pub captured_at: String,
    /// OS family (`windows`, `linux`, `macos`, …).
    pub os: String,
    /// OS version string when available.
    pub os_version: String,
    /// Host architecture (`x86_64`, …).
    pub arch: String,
    /// Number of logical CPUs reported by the OS.
    pub logical_cpus: usize,
    /// Optional governor / power hint (best-effort; may be empty).
    pub power_hint: String,
}

impl EnvironmentSnapshot {
    /// Capture current process environment (best-effort).
    pub fn capture() -> Self {
        Self {
            captured_at: chrono::Utc::now().to_rfc3339(),
            os: std::env::consts::OS.to_string(),
            os_version: os_version_string(),
            arch: std::env::consts::ARCH.to_string(),
            logical_cpus: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            power_hint: String::new(),
        }
    }
}

fn os_version_string() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("OS").unwrap_or_else(|_| "Windows".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        "unknown".into()
    }
}

/// Abstraction over real and mock hardware backends.
pub trait HardwareBackend: Send + Sync {
    /// Discover hardware.
    fn discover(&self, knowledge: &KnowledgePack) -> Result<HardwareInfo>;
}

/// Detect hardware on this host using real CPUID.
///
/// On unsupported CPUs returns `Ok(HardwareInfo)` with
/// [`SupportStatus::Unsupported`] — callers should print the message and exit
/// non-zero without panicking.
pub fn detect_hardware() -> Result<HardwareInfo> {
    let knowledge = KnowledgePack::builtin();
    detect_real_hardware(&knowledge)
}

/// Detect with an explicit knowledge pack (e.g. loaded from `profiles/amd`).
pub fn detect_hardware_with(knowledge: &KnowledgePack) -> Result<HardwareInfo> {
    detect_real_hardware(knowledge)
}

/// Require supported hardware or return a clear error.
pub fn require_supported(info: &HardwareInfo) -> Result<Microarch> {
    match &info.support {
        SupportStatus::Supported { microarch } => Ok(*microarch),
        SupportStatus::UnsupportedMicroarch { reason, .. } => {
            Err(SiliceraError::UnsupportedMicroarch(reason.clone()))
        }
        SupportStatus::Unsupported { reason } => {
            Err(SiliceraError::UnsupportedCpu(reason.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_zen5_supported() {
        let kp = KnowledgePack::builtin();
        let hw = MockHardware::zen5_dual_ccd();
        let info = hw.discover(&kp).unwrap();
        assert!(info.support.is_supported());
        assert!(info.fingerprint.is_some());
        assert!(info.is_mock);
    }

    #[test]
    fn mock_unsupported_graceful() {
        let kp = KnowledgePack::builtin();
        let hw = MockHardware::unsupported_intel();
        let info = hw.discover(&kp).unwrap();
        assert!(!info.support.is_supported());
        assert!(info.fingerprint.is_none());
    }
}
