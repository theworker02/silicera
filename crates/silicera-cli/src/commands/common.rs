//! Shared helpers for CLI command implementations.

use std::path::Path;

use anyhow::{bail, Result};
use silicera::hardware::SupportStatus;
use silicera::hnep::HnepProfile;
use silicera::specialize::DecisionNode;
use silicera::topology::format_bytes;
use silicera_lab::read_export;

pub(crate) fn exit_for_support(support: &SupportStatus) -> Result<()> {
    if support.is_supported() {
        Ok(())
    } else {
        eprintln!();
        eprintln!("Silicera cannot specialize on this CPU (Zen3/Zen4/Zen5 support only).");
        eprintln!("{}", support.message());
        eprintln!("Supported: AMD Zen3 / Zen4 / Zen5 with knowledge-pack validation.");
        std::process::exit(2);
    }
}

pub(crate) fn load_profile_or_export(path: &str) -> Result<HnepProfile> {
    let p = Path::new(path);
    if p.extension().and_then(|e| e.to_str()) == Some("json") {
        // Could be sanitized export or placeholder — try export first.
        match read_export(p) {
            Ok(exp) => Ok(exp.profile),
            Err(_) => {
                // Placeholder has no profile — surface a clear error.
                bail!(
                    "file {path} is not a sanitized split export with an embedded HNEP; \
                     pass a .hnep or machine_*.split.json"
                )
            }
        }
    } else {
        Ok(HnepProfile::read_from(p)?)
    }
}

pub(crate) fn dump_node(node: &DecisionNode, depth: usize) {
    let pad = "  ".repeat(depth + 1);
    match node {
        DecisionNode::Select { variant } => {
            println!("{pad}select → {variant}");
        }
        DecisionNode::SizeBranch {
            threshold_bytes,
            label,
            less,
            greater_or_equal,
        } => {
            println!(
                "{pad}branch {label} thr={} ({})",
                threshold_bytes,
                format_bytes(*threshold_bytes)
            );
            println!("{pad}  < :");
            dump_node(less, depth + 2);
            println!("{pad}  ≥ :");
            dump_node(greater_or_equal, depth + 2);
        }
    }
}
