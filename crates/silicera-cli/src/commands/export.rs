//! Feedback, fleet, repro, remarks, and archive export commands.

use std::path::Path;

use anyhow::{bail, Context, Result};
use silicera::archive::{entry_from_eval_json, ResultsArchive};
use silicera::feedback::CompilerFeedback;
use silicera::fleet::FleetShare;
use silicera::hardware::detect_hardware;
use silicera::hnep::HnepProfile;
use silicera::remarks::RemarksBundle;
use silicera::repro::ReproPack;

use crate::style::{header, kv};

pub fn cmd_feedback(profile: &str, output: &str, json: bool) -> Result<()> {
    let p = HnepProfile::read_from(Path::new(profile))?;
    let scf = CompilerFeedback::from_hnep(&p)?;
    if json {
        println!("{}", scf.to_json_pretty()?);
    } else {
        println!("{}", header("silicera feedback"));
        println!("{}", kv("format", &scf.format));
        println!("{}", kv("version", &scf.version.to_string()));
        println!("{}", kv("fingerprint", &scf.fingerprint));
        println!("{}", kv("workloads", &scf.workloads.len().to_string()));
        println!("{}", kv("size_classes", &scf.size_classes.len().to_string()));
        for c in &scf.caveats {
            println!("  · {c}");
        }
    }
    scf.write_to(Path::new(output))?;
    if !json {
        println!("{}", kv("wrote", output));
    }
    Ok(())
}


pub fn cmd_fleet_export(profile: &str, output: &str, opt_in: bool) -> Result<()> {
    if !opt_in {
        bail!("fleet export requires --opt-in (Silicera never uploads automatically)");
    }
    let p = HnepProfile::read_from(Path::new(profile))?;
    let share = FleetShare::from_hnep(&p, true)?;
    share.write_to(Path::new(output))?;
    println!("{}", header("silicera fleet-export"));
    println!("{}", kv("wrote", output));
    println!("{}", kv("workloads", &share.workloads.len().to_string()));
    for c in &share.caveats {
        println!("  · {c}");
    }
    Ok(())
}


pub fn cmd_repro_export(
    experiment: &str,
    output: &str,
    profile: Option<&str>,
    json: bool,
) -> Result<()> {
    let info = detect_hardware()?;
    let digest = profile
        .map(|p| HnepProfile::read_from(Path::new(p)))
        .transpose()?
        .map(|h| h.digest.hex);
    let pack = ReproPack::capture(
        &info,
        experiment,
        serde_json::json!({
            "profile": profile,
            "cli": "silicera repro-export",
        }),
        None,
        digest,
    );
    pack.write_to(Path::new(output))?;
    if json {
        println!("{}", pack.to_json_pretty()?);
    } else {
        println!("{}", header("silicera repro-export"));
        println!("{}", kv("wrote", output));
        println!("{}", kv("experiment", &pack.experiment));
        println!("{}", kv("phase", &pack.phase));
        if let Some(fp) = &pack.fingerprint {
            println!("{}", kv("fingerprint", fp));
        }
        for c in &pack.caveats {
            println!("  · {c}");
        }
    }
    Ok(())
}


pub fn cmd_remarks(profile: &str, output: &str, json: bool) -> Result<()> {
    let p = HnepProfile::read_from(Path::new(profile))?;
    let bundle = RemarksBundle::from_hnep(&p)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&bundle)?);
    } else {
        println!("{}", header("silicera remarks"));
        println!("{}", kv("format", &bundle.format));
        println!("{}", kv("remarks", &bundle.remarks.len().to_string()));
        println!("{}", kv("fingerprint", &bundle.fingerprint));
        for line in bundle.summary_lines().iter().skip(1) {
            println!("{line}");
        }
        for c in &bundle.caveats {
            println!("  · {c}");
        }
    }
    bundle.write_yaml(Path::new(output))?;
    if !json {
        println!("{}", kv("wrote", output));
    }
    Ok(())
}

/// Summarize an existing remarks YAML by pass name counts.
pub fn cmd_remarks_summary(yaml_path: &str, json: bool) -> Result<()> {
    let text = std::fs::read_to_string(yaml_path)
        .with_context(|| format!("read remarks yaml {yaml_path}"))?;
    let lines = RemarksBundle::summarize_yaml(&text);
    if json {
        // Parse "pass: n" lines into a map for tooling.
        let mut by_pass = serde_json::Map::new();
        let mut total = 0u64;
        for line in &lines {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("remarks summary:") {
                if let Some(n) = rest.split_whitespace().next().and_then(|s| s.parse().ok()) {
                    total = n;
                }
                continue;
            }
            if let Some((pass, n)) = t.split_once(':') {
                let pass = pass.trim();
                if pass.is_empty() || pass.starts_with('(') {
                    continue;
                }
                if let Ok(count) = n.trim().parse::<u64>() {
                    by_pass.insert(pass.to_string(), serde_json::json!(count));
                }
            }
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": yaml_path,
                "total": total,
                "by_pass": by_pass,
                "lines": lines,
            }))?
        );
    } else {
        println!("{}", header("silicera remarks-summary"));
        println!("{}", kv("path", yaml_path));
        for line in &lines {
            println!("  {line}");
        }
    }
    Ok(())
}


pub fn cmd_archive(eval: Option<&str>, index: &str, list: bool, json: bool) -> Result<()> {
    let path = Path::new(index);
    let mut archive = ResultsArchive::load_or_empty(path)?;
    if let Some(eval_path) = eval {
        let rel = eval_path.replace('\\', "/");
        let entry = entry_from_eval_json(Path::new(eval_path), &rel)?;
        archive.push_and_write(entry.clone(), path)?;
        println!("{}", header("silicera archive"));
        println!("{}", kv("added", &entry.id));
        println!("{}", kv("index", index));
        println!("{}", kv("summary", &entry.summary));
        for h in &entry.highlights {
            println!("  · {h}");
        }
        return Ok(());
    }
    if list || json {
        if json {
            println!("{}", serde_json::to_string_pretty(&archive)?);
        } else {
            println!("{}", header("silicera archive"));
            println!("{}", kv("entries", &archive.entries.len().to_string()));
            for e in &archive.entries {
                println!("  · {}  {}  {}", e.id, e.experiment, e.host_brand);
                println!("      {}", e.summary);
            }
        }
        return Ok(());
    }
    bail!("pass --eval <single-machine-eval.json> to add, or --list to show");
}
