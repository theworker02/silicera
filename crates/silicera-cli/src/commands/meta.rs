//! About, schema, init, runtime, report, and toolchain meta commands.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use silicera::hardware::{detect_hardware, EnvironmentSnapshot};
use silicera::hnep::{
    HnepProfile, HNEP_FORMAT, HNEP_VERSION, HNEP_VERSION_MAX_SUPPORTED, HNEP_VERSION_MIN_SUPPORTED,
};
use silicera::lifecycle::ProfileHealth;
use silicera::probe_counters;
use silicera::{
    brand_json, AFFILIATION_DISCLAIMER, BRAND_LINE, FUNDING_URL, HOMEPAGE, LICENSE, NAME, PHASE,
    RELEASE_LINE, REPOSITORY, VERSION,
};
use silicera_runtime::runtime_about;

use crate::style::{header, kv};

pub fn cmd_about(json: bool) -> Result<()> {
    if json {
        println!("{}", brand_json()?);
        return Ok(());
    }
    crate::catalog::print_banner();
    println!();
    println!("{}", header("silicera about"));
    println!("{}", kv("name", NAME));
    println!("{}", kv("version", VERSION));
    println!("{}", kv("release", RELEASE_LINE));
    println!("{}", kv("phase", PHASE));
    println!("{}", kv("license", LICENSE));
    println!("{}", kv("homepage", HOMEPAGE));
    println!("{}", kv("repository", REPOSITORY));
    println!("{}", kv("funding", FUNDING_URL));
    println!();
    println!("  {}", BRAND_LINE);
    println!("  {}", AFFILIATION_DISCLAIMER);
    Ok(())
}

/// Probe optional tools for PGO/BOLT readiness docs (presence only; never invent results).
pub fn cmd_toolchain(json: bool) -> Result<()> {
    let tools = [
        ("rustc", &["rustc", "--version"][..]),
        ("cargo", &["cargo", "--version"][..]),
        ("clang", &["clang", "--version"][..]),
        ("clang++", &["clang++", "--version"][..]),
        ("llvm-profdata", &["llvm-profdata", "--version"][..]),
        ("llvm-bolt", &["llvm-bolt", "--version"][..]),
    ];
    let mut rows = Vec::new();
    for (name, argv) in tools {
        let (present, detail) = probe_tool(argv);
        rows.push(serde_json::json!({
            "tool": name,
            "present": present,
            "detail": detail,
        }));
    }
    let doc = serde_json::json!({
        "silicera_version": VERSION,
        "release_line": RELEASE_LINE,
        "note": "Presence probe only — does not run PGO/BOLT or invent speedups.",
        "tools": rows,
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }
    println!("{}", header("silicera toolchain"));
    println!("{}", kv("note", "presence only (PGO/BOLT readiness)"));
    for row in &rows {
        let name = row["tool"].as_str().unwrap_or("?");
        let present = row["present"].as_bool().unwrap_or(false);
        let detail = row["detail"].as_str().unwrap_or("");
        let status = if present { "present" } else { "missing" };
        println!("{}", kv(name, &format!("{status} — {detail}")));
    }
    println!();
    println!("  See docs/benchmarks/pgo-bolt.md for methodology.");
    Ok(())
}

fn probe_tool(argv: &[&str]) -> (bool, String) {
    let (prog, args) = match argv.split_first() {
        Some((p, rest)) => (*p, rest),
        None => return (false, "empty argv".into()),
    };
    match Command::new(prog).args(args).output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let line = stdout.lines().next().unwrap_or("ok").trim();
            (true, line.chars().take(120).collect())
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let line = stderr.lines().next().unwrap_or("non-zero exit").trim();
            (false, line.chars().take(120).collect())
        }
        Err(e) => (false, format!("not found ({e})")),
    }
}

/// Write a professional Markdown host report (measured facts only).
pub fn cmd_report(output: &str, profile: Option<&str>, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    let counters = probe_counters();
    let tpm = silicera::tpm::TpmBindingStub::probe();
    let health = if let Some(path) = profile {
        let p = HnepProfile::read_from(Path::new(path))?;
        let live = EnvironmentSnapshot::capture();
        Some(ProfileHealth::assess_default(
            &p,
            &live,
            info.fingerprint.as_ref().map(|f| f.value.as_str()),
        )?)
    } else {
        None
    };

    let md = render_host_report(&info, &counters, &tpm, profile, health.as_ref());
    let out = PathBuf::from(output);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // UTF-8 with BOM so Windows editors / PowerShell default encoding open cleanly.
    let mut bytes = Vec::with_capacity(3 + md.len());
    bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    bytes.extend_from_slice(md.as_bytes());
    std::fs::write(&out, bytes).with_context(|| format!("write {output}"))?;

    let summary = serde_json::json!({
        "wrote": output,
        "host": info.brand,
        "supported": info.support.is_supported(),
        "fingerprint": info.fingerprint.as_ref().map(|f| &f.value),
        "profile_health": health.as_ref().map(|h| serde_json::json!({
            "score": h.score,
            "grade": h.grade.label(),
            "staleness": h.staleness,
        })),
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("{}", header("silicera report"));
        println!("{}", kv("wrote", output));
        println!("{}", kv("host", &info.brand));
        println!("{}", kv("support", &info.support.message()));
        if let Some(h) = &health {
            println!(
                "{}",
                kv(
                    "profile_health",
                    &format!("{} ({}/100)", h.grade.label(), h.score)
                )
            );
        }
    }
    Ok(())
}

fn render_host_report(
    info: &silicera::hardware::HardwareInfo,
    counters: &silicera::counters::CounterProbe,
    tpm: &silicera::tpm::TpmBindingStub,
    profile: Option<&str>,
    health: Option<&ProfileHealth>,
) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {NAME} host report\n\n"));
    md.push_str(&format!("**{BRAND_LINE}**  \n"));
    md.push_str(&format!("{RELEASE_LINE} | v{VERSION}\n\n"));
    md.push_str("## Host\n\n");
    md.push_str(&format!("| Field | Value |\n|-------|-------|\n"));
    md.push_str(&format!("| Brand | {} |\n", info.brand));
    md.push_str(&format!("| Vendor | {} |\n", info.vendor));
    md.push_str(&format!(
        "| CPUID | family={:#x} model={:#x} stepping={:#x} |\n",
        info.family, info.model, info.stepping
    ));
    md.push_str(&format!("| Support | {} |\n", info.support.message()));
    if let Some(fp) = &info.fingerprint {
        md.push_str(&format!("| Fingerprint | `{}` |\n", fp.value));
    } else {
        md.push_str("| Fingerprint | _(none -- unsupported or unavailable)_ |\n");
    }
    md.push_str(&format!(
        "| Environment | {} / {} / {} logical CPUs |\n",
        info.environment.os, info.environment.arch, info.environment.logical_cpus
    ));
    md.push_str(
        "\n> Fingerprint is machine class identity for specialization -- **not** authentication.\n\n",
    );
    md.push_str("## Topology (summary)\n\n");
    md.push_str("```\n");
    for line in info.topology.summary_lines() {
        md.push_str(&line);
        md.push('\n');
    }
    md.push_str("```\n\n");
    md.push_str("## Probes\n\n");
    md.push_str(&format!(
        "- **HW counters:** {} — {}\n",
        counters.availability.label(),
        counters.limitation
    ));
    md.push_str(&format!(
        "- **TPM binding:** {:?} — {}\n\n",
        tpm.availability, tpm.notes
    ));
    if let Some(path) = profile {
        md.push_str("## Profile health\n\n");
        md.push_str(&format!("Profile path: `{path}`\n\n"));
        if let Some(h) = health {
            md.push_str(&format!(
                "| Metric | Value |\n|--------|-------|\n| Score | {}/100 |\n| Grade | {} |\n| Integrity | {} |\n| Confidence | {} |\n| Staleness | {} |\n| Retrain | {} |\n\n",
                h.score,
                h.grade.label(),
                if h.integrity_ok { "ok" } else { "failed" },
                h.confidence,
                h.staleness,
                if h.retrain_recommended { "recommended" } else { "not needed" }
            ));
            md.push_str("Notes:\n\n");
            for n in &h.notes {
                md.push_str(&format!("- {n}\n"));
            }
            md.push('\n');
        }
    }
    md.push_str("---\n\n");
    md.push_str(&format!("*{AFFILIATION_DISCLAIMER}*  \n"));
    md.push_str(&format!("Sponsor: [{FUNDING_URL}]({FUNDING_URL})  \n"));
    md.push_str(&format!("Repository: {REPOSITORY}\n"));
    md
}


pub fn cmd_schema(json: bool) -> Result<()> {
    let doc = serde_json::json!({
        "silicera_version": VERSION,
        "phase": PHASE,
        "hnep": {
            "format": HNEP_FORMAT,
            "version": HNEP_VERSION,
            "min_supported": HNEP_VERSION_MIN_SUPPORTED,
            "max_supported": HNEP_VERSION_MAX_SUPPORTED,
        },
        "scf": { "format": "silicera-scf", "note": "see silicera feedback" },
        "remarks": { "format": "silicera-remarks-yaml", "note": "experimental LLVM-style" },
        "funding_url": FUNDING_URL,
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }
    println!("{}", header("silicera schema"));
    println!("{}", kv("silicera", VERSION));
    println!("{}", kv("phase", PHASE));
    println!("{}", kv("hnep_format", HNEP_FORMAT));
    println!("{}", kv("hnep_version", &HNEP_VERSION.to_string()));
    println!(
        "{}",
        kv(
            "hnep_supported",
            &format!("{HNEP_VERSION_MIN_SUPPORTED}..={HNEP_VERSION_MAX_SUPPORTED}")
        )
    );
    println!("{}", kv("scf", "silicera-scf (JSON)"));
    println!("{}", kv("remarks", "experimental YAML"));
    Ok(())
}


pub fn cmd_init(dir: &str) -> Result<()> {
    let root = Path::new(dir);
    std::fs::create_dir_all(root)?;
    let readme = root.join("README.md");
    if !readme.exists() {
        std::fs::write(
            &readme,
            format!(
                "# Silicera local outputs\n\n\
                 Generated by `silicera init`.\n\n\
                 - Place `.hnep` profiles here\n\
                 - Single-machine eval JSON + repro sidecars\n\
                 - SCF / remarks exports\n\n\
                 {RELEASE_LINE} | {BRAND_LINE}\n\
                 Funding: {FUNDING_URL}\n\
                 {AFFILIATION_DISCLAIMER}\n"
            ),
        )?;
    }
    let gitkeep = root.join(".gitkeep");
    if !gitkeep.exists() {
        std::fs::write(&gitkeep, "")?;
    }
    println!("{}", header("silicera init"));
    println!("{}", kv("dir", dir));
    println!("{}", kv("readme", &readme.display().to_string()));
    Ok(())
}


pub fn cmd_runtime(json: bool) -> Result<()> {
    let info = runtime_about();
    if json {
        println!("{}", info.to_json_pretty()?);
        return Ok(());
    }
    println!("{}", header("silicera runtime"));
    println!("{}", kv("name", &info.name));
    println!("{}", kv("version", &info.version));
    println!("{}", kv("phase", &info.phase));
    println!("{}", kv("license", &info.license));
    println!("{}", kv("homepage", &info.homepage));
    println!("{}", kv("repository", &info.repository));
    println!("{}", kv("funding", &info.funding_url));
    println!();
    println!("  {}", info.brand_line);
    println!("  {}", info.affiliation);
    Ok(())
}
