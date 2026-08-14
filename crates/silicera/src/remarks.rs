//! LLVM-style YAML remarks derived from SCF / HNEP (experimental).
//!
//! Emits a text document compatible in *spirit* with LLVM optimization remarks:
//! pass name, function/workload key, and an argument map. Not a binary bitstream
//! remark format — intended for human review and future tooling bridges.
//!
//! See `docs/research/llvm-integration.md`.

use serde::{Deserialize, Serialize};

use crate::feedback::CompilerFeedback;
use crate::hnep::HnepProfile;
use crate::Result;

/// Format id for Silicera remark bundles.
pub const REMARKS_FORMAT: &str = "silicera-llvm-remarks";

/// Schema version.
pub const REMARKS_VERSION: u32 = 1;

/// One remark (YAML document in a multi-doc stream).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remark {
    /// Pass name (stable for tooling).
    pub pass: String,
    /// Remark kind.
    pub remark_name: String,
    /// Workload / function surrogate key.
    pub function: String,
    /// Arguments (stringly for YAML friendliness).
    pub args: Vec<RemarkArg>,
}

/// Key/value argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemarkArg {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
}

/// Bundle of remarks plus metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemarksBundle {
    /// Format magic.
    pub format: String,
    /// Schema version.
    pub version: u32,
    /// Producer version.
    pub silicera_version: String,
    /// Fingerprint.
    pub fingerprint: String,
    /// Remarks.
    pub remarks: Vec<Remark>,
    /// Caveats.
    pub caveats: Vec<String>,
}

impl RemarksBundle {
    /// Build from SCF.
    pub fn from_scf(scf: &CompilerFeedback) -> Self {
        let mut remarks = Vec::new();
        for w in &scf.workloads {
            remarks.push(Remark {
                pass: "silicera-specialize".into(),
                remark_name: "Passed".into(),
                function: w.name.clone(),
                args: vec![
                    arg("Winner", &w.winner),
                    arg("Confidence", &w.confidence),
                    arg(
                        "WinnerMedianNs",
                        &w.winner_median_ns
                            .map(|n| format!("{n:.0}"))
                            .unwrap_or_else(|| "n/a".into()),
                    ),
                    arg(
                        "BaselineMedianNs",
                        &w.baseline_median_ns
                            .map(|n| format!("{n:.0}"))
                            .unwrap_or_else(|| "n/a".into()),
                    ),
                    arg("Fingerprint", &scf.fingerprint),
                ],
            });
        }
        for sc in &scf.size_classes {
            remarks.push(Remark {
                pass: "silicera-size-class".into(),
                remark_name: "Passed".into(),
                function: format!("size_class.{}", sc.class),
                args: vec![
                    arg("Class", &sc.class),
                    arg("Winner", &sc.winner),
                    arg("Confidence", &sc.confidence),
                    arg("ThresholdBytes", &sc.threshold_bytes.to_string()),
                    arg("WorkingSetBytes", &sc.working_set_bytes.to_string()),
                ],
            });
        }
        Self {
            format: REMARKS_FORMAT.into(),
            version: REMARKS_VERSION,
            silicera_version: crate::VERSION.into(),
            fingerprint: scf.fingerprint.clone(),
            remarks,
            caveats: vec![
                "Experimental YAML remarks — not LLVM bitstream remarks.".into(),
                "Winners are machine-local; re-verify on the consuming host.".into(),
                "Treat as candidate hints for multiversioning / ordering, not mandates.".into(),
            ],
        }
    }

    /// Build from HNEP (via SCF).
    pub fn from_hnep(profile: &HnepProfile) -> Result<Self> {
        let scf = CompilerFeedback::from_hnep(profile)?;
        Ok(Self::from_scf(&scf))
    }

    /// Emit multi-document YAML (LLVM remark–like).
    pub fn to_yaml_stream(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# {} v{}  silicera={}  fingerprint={}\n",
            self.format, self.version, self.silicera_version, self.fingerprint
        ));
        for c in &self.caveats {
            out.push_str(&format!("# caveat: {c}\n"));
        }
        out.push('\n');
        for r in &self.remarks {
            out.push_str("--- !Passed\n");
            out.push_str(&format!("Pass: {}\n", r.pass));
            out.push_str(&format!("Name: {}\n", r.remark_name));
            out.push_str(&format!("Function: {}\n", yaml_quote(&r.function)));
            out.push_str("Args:\n");
            for a in &r.args {
                out.push_str(&format!("  - {}: {}\n", a.key, yaml_quote(&a.value)));
            }
            out.push('\n');
        }
        out
    }

    /// Write YAML stream to path.
    pub fn write_yaml(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_yaml_stream())?;
        Ok(())
    }

    /// Summarize remark counts by pass name (consumer / ops surface).
    pub fn summary_lines(&self) -> Vec<String> {
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &self.remarks {
            *counts.entry(r.pass.as_str()).or_default() += 1;
        }
        let mut lines = Vec::new();
        lines.push(format!(
            "remarks: {} total · format {} v{} · fingerprint {}",
            self.remarks.len(),
            self.format,
            self.version,
            self.fingerprint
        ));
        if counts.is_empty() {
            lines.push("  (no remarks)".into());
        } else {
            for (pass, n) in counts {
                lines.push(format!("  {pass}: {n}"));
            }
        }
        for c in &self.caveats {
            lines.push(format!("  caveat: {c}"));
        }
        lines
    }

    /// Parse a Silicera YAML remarks stream (best-effort; counts `Pass:` lines).
    pub fn summarize_yaml(yaml: &str) -> Vec<String> {
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut total = 0usize;
        let mut fingerprint = String::from("?");
        for line in yaml.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("# silicera-llvm-remarks") {
                let _ = rest;
            }
            if t.starts_with("#") && t.contains("fingerprint=") {
                if let Some(idx) = t.find("fingerprint=") {
                    fingerprint = t[idx + "fingerprint=".len()..].trim().to_string();
                }
            }
            if let Some(pass) = t.strip_prefix("Pass:") {
                let pass = pass.trim().to_string();
                if !pass.is_empty() {
                    *counts.entry(pass).or_default() += 1;
                    total += 1;
                }
            }
        }
        let mut lines = Vec::new();
        lines.push(format!(
            "remarks summary: {total} Pass entries · fingerprint {fingerprint}"
        ));
        if counts.is_empty() {
            lines.push("  (no Pass: lines found)".into());
        } else {
            for (pass, n) in counts {
                lines.push(format!("  {pass}: {n}"));
            }
        }
        lines
    }
}

fn arg(key: &str, value: &str) -> RemarkArg {
    RemarkArg {
        key: key.into(),
        value: value.into(),
    }
}

fn yaml_quote(s: &str) -> String {
    if s.chars()
        .any(|c| c.is_whitespace() || matches!(c, ':' | '#' | '{' | '}' | '[' | ']' | ',' | '"'))
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::{CompilerFeedback, ScfWorkload, SCF_FORMAT, SCF_VERSION};

    #[test]
    fn yaml_contains_pass() {
        let scf = CompilerFeedback {
            format: SCF_FORMAT.into(),
            version: SCF_VERSION,
            silicera_version: "0.1.0".into(),
            fingerprint: "SLC:AMD:ZEN5:00:00:00:aaaaaaaaaaaaaaaa:bbbbbbbbbbbbbbbb".into(),
            host_brand: None,
            microarchitecture: Some("zen5".into()),
            workloads: vec![ScfWorkload {
                name: "concurrency".into(),
                winner: "candidate".into(),
                confidence: "HIGH".into(),
                baseline: None,
                winner_median_ns: Some(1.0),
                baseline_median_ns: Some(2.0),
                rationale: "test".into(),
            }],
            size_classes: vec![],
            fallback_variant: None,
            caveats: vec![],
        };
        let bundle = RemarksBundle::from_scf(&scf);
        let y = bundle.to_yaml_stream();
        assert!(y.contains("Pass: silicera-specialize"));
        assert!(y.contains("Function: concurrency"));
        assert!(y.contains("Winner: candidate"));
        let summary = bundle.summary_lines();
        assert!(summary[0].contains("1 total"));
        assert!(summary.iter().any(|l| l.contains("silicera-specialize: 1")));
        let parsed = RemarksBundle::summarize_yaml(&y);
        assert!(parsed.iter().any(|l| l.contains("silicera-specialize: 1")));
    }
}
