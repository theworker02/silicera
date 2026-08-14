//! Machine discovery, health, and environment commands.

use std::path::Path;

use anyhow::{bail, Context, Result};
use silicera::hardware::{
    detect_hardware, detect_hardware_with, EnvironmentSnapshot, HardwareBackend, MockHardware,
};
use silicera::hnep::HnepProfile;
use silicera::knowledge::KnowledgePack;
use silicera::retrain::plan_partial_retrain;
use silicera::staleness::{assess_staleness, StalenessPolicy};
use silicera::topology::format_bytes;
use silicera::{probe_counters, BRAND_LINE, FUNDING_URL, PHASE, VERSION};

use super::common::exit_for_support;
use crate::style::{header, kv, rule};

pub fn cmd_inspect(json: bool, mock: Option<&str>) -> Result<()> {
    let info = match mock {
        Some("zen4") => MockHardware::zen4_single_ccd().discover(&KnowledgePack::builtin())?,
        Some("zen5") => MockHardware::zen5_dual_ccd().discover(&KnowledgePack::builtin())?,
        Some("unsupported") => {
            MockHardware::unsupported_intel().discover(&KnowledgePack::builtin())?
        }
        Some(other) => bail!("unknown mock '{other}' (zen4|zen5|unsupported)"),
        None => detect_hardware()?,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return exit_for_support(&info.support);
    }

    println!("{}", header("silicera inspect"));
    println!("{}", kv("brand", &info.brand));
    println!("{}", kv("vendor", &info.vendor));
    println!(
        "{}",
        kv(
            "cpuid",
            &format!(
                "family={:#x} model={:#x} stepping={:#x}",
                info.family, info.model, info.stepping
            )
        )
    );
    println!("{}", kv("support", &info.support.message()));
    if let Some(fp) = &info.fingerprint {
        println!("{}", kv("fingerprint", &fp.value));
    }
    println!("{}", kv("mock", &info.is_mock.to_string()));
    println!();
    println!("  topology");
    for line in info.topology.summary_lines() {
        println!("    {line}");
    }
    if let Some(v) = &info.validation {
        if !v.warnings.is_empty() {
            println!();
            println!("  knowledge-pack warnings:");
            for w in &v.warnings {
                println!("    - {w}");
            }
        }
        if !v.errors.is_empty() {
            println!();
            println!("  knowledge-pack errors:");
            for e in &v.errors {
                println!("    - {e}");
            }
        }
    }
    println!("{}", rule());
    println!("  note: fingerprint is NOT authentication (see docs/security/fingerprint.md)");
    exit_for_support(&info.support)
}


pub fn cmd_probe(packs_dir: &str, json: bool) -> Result<()> {
    let kp = KnowledgePack::load_dir(Path::new(packs_dir))
        .with_context(|| format!("load packs from {packs_dir}"))?;
    let info = detect_hardware_with(&kp)?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "packs": kp.packs.iter().map(|p| {
                    serde_json::json!({
                        "microarch": p.microarch.tag(),
                        "name": p.name,
                        "families": p.families,
                        "provenance": p.provenance,
                    })
                }).collect::<Vec<_>>(),
                "host": info,
            })
        );
        return exit_for_support(&info.support);
    }

    println!("{}", header("silicera probe"));
    println!("{}", kv("packs_dir", packs_dir));
    println!("{}", kv("packs_loaded", &kp.packs.len().to_string()));
    for p in &kp.packs {
        println!(
            "  · {:<6} {}  families={:?}  L2={}  L3=[{}, {}]",
            p.microarch.tag(),
            p.name,
            p.families.iter().map(|f| format!("{f:#x}")).collect::<Vec<_>>(),
            format_bytes(p.caches.l2_bytes),
            format_bytes(p.caches.l3_per_ccd_min),
            format_bytes(p.caches.l3_per_ccd_max)
        );
    }
    println!();
    println!("{}", kv("host", &info.brand));
    println!("{}", kv("support", &info.support.message()));
    if let Some(fp) = &info.fingerprint {
        println!("{}", kv("fingerprint", &fp.value));
    }
    exit_for_support(&info.support)
}


pub fn cmd_doctor(json: bool, profile: Option<&str>) -> Result<()> {
    let info = detect_hardware()?;
    let tpm = silicera::tpm::TpmBindingStub::probe();
    let counters = probe_counters();
    let stale_json = if let Some(path) = profile {
        let p = HnepProfile::read_from(Path::new(path))?;
        let live = EnvironmentSnapshot::capture();
        let report = assess_staleness(
            &p,
            &live,
            info.fingerprint.as_ref().map(|f| f.value.as_str()),
            &StalenessPolicy::default(),
        );
        let plan = plan_partial_retrain(&p, &report.signals);
        Some(serde_json::json!({
            "path": path,
            "staleness": report,
            "partial_retrain": plan,
        }))
    } else {
        None
    };
    let report = serde_json::json!({
        "silicera_version": VERSION,
        "phase": PHASE,
        "brand_line": BRAND_LINE,
        "host_supported": info.support.is_supported(),
        "support_message": info.support.message(),
        "brand": info.brand,
        "fingerprint": info.fingerprint.as_ref().map(|f| &f.value),
        "os": info.environment.os,
        "arch": info.environment.arch,
        "logical_cpus": info.environment.logical_cpus,
        "tpm": tpm,
        "hw_counters": counters,
        "profile_check": stale_json,
        "affiliation": "Not affiliated with, endorsed by, or certified by Advanced Micro Devices, Inc.",
        "funding_url": FUNDING_URL,
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", header("silicera doctor"));
        println!("{}", kv("version", VERSION));
        println!("{}", kv("phase", PHASE));
        println!("{}", kv("host", &info.brand));
        println!("{}", kv("support", &info.support.message()));
        if let Some(fp) = &info.fingerprint {
            println!("{}", kv("fingerprint", &fp.value));
        }
        println!(
            "{}",
            kv(
                "env",
                &format!(
                    "{} / {} / {} cpus",
                    info.environment.os, info.environment.arch, info.environment.logical_cpus
                )
            )
        );
        println!("{}", kv("tpm", &format!("{:?}", tpm.availability)));
        println!(
            "{}",
            kv(
                "hw_counters",
                &format!("{} — {}", counters.availability.label(), counters.limitation)
            )
        );
        if let Some(path) = profile {
            let p = HnepProfile::read_from(Path::new(path))?;
            let live = EnvironmentSnapshot::capture();
            let stale = assess_staleness(
                &p,
                &live,
                info.fingerprint.as_ref().map(|f| f.value.as_str()),
                &StalenessPolicy::default(),
            );
            println!();
            println!("{}", kv("profile", path));
            println!("{}", kv("staleness", stale.severity.label()));
            for s in &stale.signals {
                println!("  · {} [{}]: {}", s.field, s.severity.label(), s.note);
            }
            if stale.retrain.recommended {
                println!("{}", kv("retrain", &stale.retrain.summary));
                println!("{}", kv("suggest", &stale.retrain.suggested_command));
                let plan = plan_partial_retrain(&p, &stale.signals);
                if plan.is_partial {
                    println!("{}", kv("partial", &plan.only_flag));
                }
            } else {
                println!("{}", kv("retrain", "not needed"));
            }
        } else {
            println!();
            println!("  tip: silicera doctor --profile out/profile.hnep");
            println!("  tip: silicera calm-check && silicera eval -o out/single-machine-eval.json");
            println!("  tip: silicera export remarks out/profile.hnep -o out/remarks.yaml");
            println!("  tip: silicera commands   ·   silicera guide quickstart");
        }
        println!();
        println!("  checklist (single-Zen)");
        println!("  · inspect / probe / train / verify --spot-check");
        println!("  · harness · native-artifacts --suite · align · placement · threads");
        println!("  · feedback (SCF) · remarks (LLVM YAML) · archive");
        println!("  deferred: Silicon Split Machine B (needs second Zen host)");
        println!();
        println!("  {}", BRAND_LINE);
        println!("  Not affiliated with AMD.");
        println!("  funding  {FUNDING_URL}");
    }
    if !info.support.is_supported() {
        std::process::exit(2);
    }
    Ok(())
}

pub fn cmd_fingerprint(json: bool, mock: Option<&str>) -> Result<()> {
    let info = match mock {
        Some("zen4") => MockHardware::zen4_single_ccd().discover(&KnowledgePack::builtin())?,
        Some("zen5") => MockHardware::zen5_dual_ccd().discover(&KnowledgePack::builtin())?,
        Some("unsupported") => {
            MockHardware::unsupported_intel().discover(&KnowledgePack::builtin())?
        }
        Some(other) => bail!("unknown mock '{other}' (zen4|zen5|unsupported)"),
        None => detect_hardware()?,
    };
    let fp = info.fingerprint.as_ref().map(|f| f.value.as_str());
    if json {
        println!(
            "{}",
            serde_json::json!({
                "fingerprint": fp,
                "brand": info.brand,
                "supported": info.support.is_supported(),
                "note": "fingerprint is NOT authentication",
            })
        );
        return exit_for_support(&info.support);
    }
    println!("{}", header("silicera fingerprint"));
    match fp {
        Some(v) => println!("{}", kv("fingerprint", v)),
        None => println!("{}", kv("fingerprint", "(none — host unsupported)")),
    }
    println!("{}", kv("brand", &info.brand));
    println!();
    println!("  note: fingerprint is NOT authentication (see docs/security/fingerprint.md)");
    exit_for_support(&info.support)
}


pub fn cmd_topology(json: bool, mock: Option<&str>) -> Result<()> {
    let info = match mock {
        Some("zen4") => MockHardware::zen4_single_ccd().discover(&KnowledgePack::builtin())?,
        Some("zen5") => MockHardware::zen5_dual_ccd().discover(&KnowledgePack::builtin())?,
        Some(other) => bail!("unknown mock '{other}' (zen4|zen5)"),
        None => detect_hardware()?,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&info.topology)?);
        return exit_for_support(&info.support);
    }
    println!("{}", header("silicera topology"));
    println!("{}", kv("brand", &info.brand));
    println!("{}", kv("threads", &info.topology.thread_count().to_string()));
    println!("{}", kv("cores", &info.topology.core_count().to_string()));
    println!();
    for line in info.topology.summary_lines() {
        println!("  {line}");
    }
    exit_for_support(&info.support)
}


pub fn cmd_env(json: bool) -> Result<()> {
    let snap = EnvironmentSnapshot::capture();
    if json {
        println!("{}", serde_json::to_string_pretty(&snap)?);
        return Ok(());
    }
    println!("{}", header("silicera env"));
    println!("{}", kv("captured_at", &snap.captured_at));
    println!("{}", kv("os", &snap.os));
    println!("{}", kv("os_version", &snap.os_version));
    println!("{}", kv("arch", &snap.arch));
    println!("{}", kv("logical_cpus", &snap.logical_cpus.to_string()));
    if !snap.power_hint.is_empty() {
        println!("{}", kv("power_hint", &snap.power_hint));
    }
    Ok(())
}


pub fn cmd_packs(action: &str, packs_dir: Option<&str>, name: Option<&str>, json: bool) -> Result<()> {
    let kp = if let Some(dir) = packs_dir {
        KnowledgePack::load_dir(Path::new(dir))?
    } else {
        KnowledgePack::builtin()
    };
    match action {
        "list" => {
            if json {
                let rows: Vec<_> = kp
                    .packs
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "microarch": p.microarch.tag(),
                            "name": p.name,
                            "families": p.families,
                            "provenance": p.provenance,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                println!("{}", header("silicera packs list"));
                for p in &kp.packs {
                    println!(
                        "  · {:<6} {}  families={:?}",
                        p.microarch.tag(),
                        p.name,
                        p.families.iter().map(|f| format!("{f:#x}")).collect::<Vec<_>>()
                    );
                }
            }
            Ok(())
        }
        "show" => {
            let tag = name.context("packs show requires a microarch name (zen3|zen4|zen5)")?;
            let p = kp
                .packs
                .iter()
                .find(|p| p.microarch.tag().eq_ignore_ascii_case(tag) || p.name.eq_ignore_ascii_case(tag))
                .with_context(|| format!("no pack matching '{tag}'"))?;
            if json {
                println!("{}", serde_json::to_string_pretty(p)?);
            } else {
                println!("{}", header("silicera packs show"));
                println!("{}", kv("microarch", p.microarch.tag()));
                println!("{}", kv("name", &p.name));
                println!("{}", kv("provenance", &p.provenance));
                println!("{}", kv("L2", &format_bytes(p.caches.l2_bytes)));
                println!(
                    "{}",
                    kv(
                        "L3",
                        &format!(
                            "[{}, {}]",
                            format_bytes(p.caches.l3_per_ccd_min),
                            format_bytes(p.caches.l3_per_ccd_max)
                        )
                    )
                );
                if !p.notes.is_empty() {
                    println!("  notes: {}", p.notes);
                }
            }
            Ok(())
        }
        other => bail!("unknown packs action '{other}' (list|show)"),
    }
}


pub fn cmd_counters(json: bool) -> Result<()> {
    let c = probe_counters();
    if json {
        println!("{}", serde_json::to_string_pretty(&c)?);
        return Ok(());
    }
    println!("{}", header("silicera counters"));
    println!("{}", kv("availability", c.availability.label()));
    println!("{}", kv("platform", &c.platform));
    println!("{}", kv("limitation", &c.limitation));
    println!("{}", kv("fallback", &c.fallback));
    Ok(())
}


pub fn cmd_tpm(json: bool) -> Result<()> {
    let t = silicera::tpm::TpmBindingStub::probe();
    if json {
        println!("{}", serde_json::to_string_pretty(&t)?);
        return Ok(());
    }
    println!("{}", header("silicera tpm"));
    println!("{}", kv("availability", &format!("{:?}", t.availability)));
    println!("  {}", t.notes);
    Ok(())
}
