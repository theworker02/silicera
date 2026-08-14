//! Optional experimental TPM binding for hardware-bound profile signatures.
//!
//! **Status:** stubbed. TPM binding is intentionally separate from the
//! specialization pipeline and does not block profile generation or dispatch.
//!
//! See `docs/security/tpm.md`.
//!
//! Fingerprints are **not** authentication. TPM attestation, when implemented,
//! would only bind a profile signature to a platform — it does not replace
//! measurement correctness.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SiliceraError};
use crate::hnep::IntegrityDigest;

/// TPM binding status on this host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TpmAvailability {
    /// TPM APIs not compiled / not attempted.
    Stubbed,
    /// Probed and unavailable.
    Unavailable,
    /// Present but binding not yet implemented in this release.
    PresentUnbound,
}

/// Experimental signature envelope (stub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmBindingStub {
    /// Availability.
    pub availability: TpmAvailability,
    /// Notes for operators.
    pub notes: String,
    /// Optional digest that *would* be bound (echo of HNEP digest).
    pub profile_digest: Option<IntegrityDigest>,
}

impl TpmBindingStub {
    /// Probe TPM availability (this release: always stubbed on most platforms).
    pub fn probe() -> Self {
        Self {
            availability: TpmAvailability::Stubbed,
            notes: "TPM profile binding is an optional experiment stub. \
                    Specialization does not depend on TPM. On Windows, full TPM 2.0 \
                    attestation APIs are not wired; see docs/security/tpm.md."
                .into(),
            profile_digest: None,
        }
    }

    /// Attempt to bind a digest (stub — returns clear error).
    pub fn bind_digest(&self, digest: &IntegrityDigest) -> Result<TpmBindingStub> {
        let _ = digest;
        Err(SiliceraError::Internal(
            "TPM binding not implemented (stub only); specialization unaffected".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_stubbed() {
        let s = TpmBindingStub::probe();
        assert_eq!(s.availability, TpmAvailability::Stubbed);
    }
}
