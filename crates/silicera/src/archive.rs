//! Curated results archive entries (sanitized, measured-only).
//!
//! Stores pointers to eval JSON / HNEP digests for the research site — never
//! invents performance numbers.

use serde::{Deserialize, Serialize};

use crate::brand::PHASE;
use crate::Result;

/// Archive format id.
pub const ARCHIVE_FORMAT: &str = "silicera-results-archive";

/// Schema version.
pub const ARCHIVE_VERSION: u32 = 1;

/// One archived measurement campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// Stable id (e.g. date + short hash).
    pub id: String,
    /// RFC3339 capture time.
    pub captured_at: String,
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: String,
    /// Silicera version.
    pub silicera_version: String,
    /// Phase marker when recorded.
    #[serde(default = "default_phase")]
    pub phase: String,
    /// Experiment label (`single-machine-eval`, `native-artifacts`, …).
    pub experiment: String,
    /// Relative path to full JSON artifact in-repo (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    /// Optional HNEP digest hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hnep_digest: Option<String>,
    /// Short human summary (measured facts only).
    pub summary: String,
    /// Key metrics as plain strings (no invented %).
    #[serde(default)]
    pub highlights: Vec<String>,
}

fn default_phase() -> String {
    PHASE.into()
}

/// Archive index file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsArchive {
    /// Format magic.
    pub format: String,
    /// Schema version.
    pub version: u32,
    /// Entries newest-first preferred.
    pub entries: Vec<ArchiveEntry>,
    /// Caveats.
    pub caveats: Vec<String>,
}

impl ResultsArchive {
    /// Empty archive.
    pub fn empty() -> Self {
        Self {
            format: ARCHIVE_FORMAT.into(),
            version: ARCHIVE_VERSION,
            entries: Vec::new(),
            caveats: vec![
                "Only measured campaigns. Do not invent Machine B or speedups.".into(),
                "Fingerprints identify specialization environments — not authentication.".into(),
            ],
        }
    }

    /// Load from path or create empty.
    pub fn load_or_empty(path: &std::path::Path) -> Result<Self> {
        if path.is_file() {
            let text = std::fs::read_to_string(path)?;
            let a: ResultsArchive = serde_json::from_str(&text)?;
            Ok(a)
        } else {
            Ok(Self::empty())
        }
    }

    /// Prepend an entry and write.
    pub fn push_and_write(&mut self, entry: ArchiveEntry, path: &std::path::Path) -> Result<()> {
        self.entries.insert(0, entry);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// Build an archive entry from a single-machine eval JSON file.
pub fn entry_from_eval_json(
    path: &std::path::Path,
    relative_artifact: &str,
) -> Result<ArchiveEntry> {
    let text = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    let host = v
        .get("host_brand")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let fp = v
        .get("fingerprint")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let ver = v
        .get("silicera_version")
        .and_then(|x| x.as_str())
        .unwrap_or(crate::VERSION)
        .to_string();
    let summary = v
        .get("summary")
        .and_then(|x| x.as_str())
        .unwrap_or("single-machine-eval")
        .to_string();
    let calm = v
        .pointer("/calm/calm")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    let native_wins = v
        .pointer("/artifacts/native_win_count")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let portable_wins = v
        .pointer("/artifacts/portable_win_count")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let short: String = fp.chars().rev().take(8).collect::<String>().chars().rev().collect();
    let id = format!("eval-{}-{short}", chrono::Utc::now().format("%Y%m%d"));
    Ok(ArchiveEntry {
        id,
        captured_at: chrono::Utc::now().to_rfc3339(),
        host_brand: host,
        fingerprint: fp,
        silicera_version: ver,
        phase: v
            .get("phase")
            .and_then(|x| x.as_str())
            .unwrap_or(PHASE)
            .to_string(),
        experiment: "single-machine-eval".into(),
        artifact_path: Some(relative_artifact.into()),
        hnep_digest: None,
        summary,
        highlights: vec![
            format!("calm={calm}"),
            format!("artifact_native_wins={native_wins}"),
            format!("artifact_portable_wins={portable_wins}"),
        ],
    })
}
