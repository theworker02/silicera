//! Project brand constants — single source of truth for CLI, lab, runtime, and tooling.
//!
//! No vendor affiliation claims. Funding links are opt-in sponsor paths only.

use serde::Serialize;

/// Product name.
pub const NAME: &str = "Silicera";

/// Library / binary version (semver from Cargo).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Phase identifier for this release line (internal / changelog marker).
pub const PHASE: &str = "II";

/// Human-facing release line (prefer this over “Phase …” in user-facing copy).
pub const RELEASE_LINE: &str = "Single-Zen Research Release";

/// One-line tagline (systems / research tone).
pub const TAGLINE: &str =
    "experimental hardware-native execution research for AMD Zen";

/// Brand line used in CLI, lab, and docs (no vendor affiliation claim).
pub const BRAND_LINE: &str =
    "Silicera — experimental hardware-native execution research for AMD Zen";

/// Dual-license SPDX expression.
pub const LICENSE: &str = "MIT OR Apache-2.0";

/// Homepage from package metadata (`silicera.dev` when published).
pub const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

/// Source repository URL from package metadata.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// Primary funding / thanks.dev URL (registered path `u/gh/theworker02`).
pub const THANKS_DEV: &str = "https://thanks.dev/u/gh/theworker02";

/// Alias for tooling that expects `FUNDING_URL`.
pub const FUNDING_URL: &str = THANKS_DEV;

/// GitHub Sponsors username when configured (same as thanks.dev path owner).
pub const GITHUB_SPONSOR: &str = "theworker02";

/// Relative path from a crate README to the workspace logo mark.
pub const LOGO_RELATIVE: &str = "../../assets/logo.svg";

/// Short disclaimer — not an AMD product.
pub const AFFILIATION_DISCLAIMER: &str =
    "Not affiliated with, endorsed by, or certified by Advanced Micro Devices, Inc.";

/// Serializable brand snapshot for JSON tooling / repro sidecars.
#[derive(Debug, Clone, Serialize)]
pub struct BrandInfo {
    /// Product name.
    pub name: &'static str,
    /// Semver.
    pub version: &'static str,
    /// Phase marker.
    pub phase: &'static str,
    /// Human-facing release line.
    pub release_line: &'static str,
    /// Tagline.
    pub tagline: &'static str,
    /// Full brand line.
    pub brand_line: &'static str,
    /// SPDX license.
    pub license: &'static str,
    /// Homepage URL.
    pub homepage: &'static str,
    /// Repository URL.
    pub repository: &'static str,
    /// thanks.dev / funding URL.
    pub funding_url: &'static str,
    /// Affiliation disclaimer.
    pub affiliation: &'static str,
    /// Docs-relative logo path constant.
    pub logo_relative: &'static str,
}

impl BrandInfo {
    /// Capture current brand constants.
    pub fn current() -> Self {
        Self {
            name: NAME,
            version: VERSION,
            phase: PHASE,
            release_line: RELEASE_LINE,
            tagline: TAGLINE,
            brand_line: BRAND_LINE,
            license: LICENSE,
            homepage: HOMEPAGE,
            repository: REPOSITORY,
            funding_url: FUNDING_URL,
            affiliation: AFFILIATION_DISCLAIMER,
            logo_relative: LOGO_RELATIVE,
        }
    }

    /// Pretty-printed JSON.
    pub fn to_json_pretty(&self) -> crate::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Export brand constants as pretty JSON (CLI / CI tooling).
pub fn brand_json() -> crate::Result<String> {
    BrandInfo::current().to_json_pretty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_json_roundtrip_fields() {
        let j = brand_json().unwrap();
        assert!(j.contains("Silicera"));
        assert!(j.contains("II"));
        assert!(j.contains("Single-Zen Research Release"));
        assert!(j.contains("thanks.dev/u/gh/theworker02"));
        assert!(j.contains("MIT OR Apache-2.0"));
        assert!(j.contains("../../assets/logo.svg"));
    }

    #[test]
    fn funding_aliases_match() {
        assert_eq!(THANKS_DEV, FUNDING_URL);
        assert!(THANKS_DEV.ends_with("/u/gh/theworker02"));
        assert_eq!(GITHUB_SPONSOR, "theworker02");
        assert_eq!(PHASE, "II");
        assert_eq!(LOGO_RELATIVE, "../../assets/logo.svg");
    }
}
