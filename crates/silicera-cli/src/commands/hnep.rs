//! HNEP train, verify, explain, and decision-tree commands.

use std::path::Path;

use anyhow::{bail, Context, Result};
use silicera::hardware::{detect_hardware, EnvironmentSnapshot};
use silicera::hnep::HnepProfile;
use silicera::lifecycle::ProfileHealth;
use silicera::measure::MeasurementConfig;
use silicera::retrain::plan_partial_retrain;
use silicera::specialize::{plan_size_specialization, SpecializeConfig};
use silicera::staleness::{assess_staleness, DriftSeverity, StalenessPolicy};
use silicera::topology::format_bytes;
use silicera_lab::profile_gen::{
    generate_profile, measure_dispatch_overhead_report, size_class_winner_story, ProfileGenConfig,
};
use silicera_lab::spot_check_profile;
use silicera_runtime::{Dispatcher, MismatchPolicy};

use super::common::dump_node;
use crate::style::{header, kv, tag};

pub fn cmd_train(output: &str, iterations: usize, label: &str, only: Option<&str>) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    println!("{}", header("silicera train"));
    println!("{}", kv("host", &info.brand));
    if let Some(fp) = &info.fingerprint {
        println!("{}", kv("fingerprint", &fp.value));
    }
    println!("{}", kv("output", output));
    println!("{}", kv("iterations", &iterations.to_string()));
    if let Some(o) = only {
        println!("{}", kv("only", o));
    }
    println!();
    println!("  running tournaments (correctness + regression gates + size classes)…");

    let cfg = ProfileGenConfig {
        measurement: MeasurementConfig {
            warmup: 3,
            iterations,
            ..Default::default()
        },
        label: label.into(),
        size_tree: true,
        size_class_tournaments: true,
        extended_size_targets: only.is_none(),
        min_improvement: 0.03,
        only: only.map(|s| s.to_string()),
    };
    let (profile, results) = generate_profile(&info, &cfg, Path::new(output))?;
    for r in &results {
        println!(
            "  · {:<20} winner={:<12} confidence={}",
            r.name,
            r.winner,
            r.confidence.label()
        );
        println!("      {}", r.rationale);
    }
    if !profile.size_classes.is_empty() {
        println!();
        println!("  size classes (measured → decision tree):");
        for sc in &profile.size_classes {
            println!(
                "  · {:<6} winner={:<12} ws={} thr={} {}",
                sc.class,
                sc.winner,
                format_bytes(sc.working_set_bytes),
                format_bytes(sc.threshold_bytes),
                sc.confidence.label()
            );
        }
        let story = size_class_winner_story(&profile.size_classes);
        println!("  {}", story.summary);
    }
    if let Some(tree) = &profile.decision_tree {
        let report = measure_dispatch_overhead_report(
            tree,
            &MeasurementConfig {
                warmup: 5,
                iterations: iterations.max(50),
                ..Default::default()
            },
        )?;
        println!();
        println!("  dispatch overhead (tree evaluate vs direct stand-in):");
        println!(
            "  · batch direct_median_ns={:.0}  tree_median_ns={:.0}  delta_ns={:.0}",
            report.batch_direct.median_ns,
            report.batch_tree.median_ns,
            report.batch_delta_ns
        );
        println!(
            "  · per-size delta median={:.0} mean={:.0} min={:.0} max={:.0} ns (n={})",
            report.delta_median_ns,
            report.delta_mean_ns,
            report.delta_min_ns,
            report.delta_max_ns,
            report.per_size.len()
        );
        for s in &report.per_size {
            println!(
                "    size={:<10} direct={:.0} tree={:.0} delta={:.0}",
                s.size_bytes, s.direct_median_ns, s.tree_median_ns, s.delta_ns
            );
        }
    }
    println!();
    println!("{}", kv("wrote", output));
    println!("{}", kv("digest", &profile.digest.hex));
    println!("{}", kv("hnep_version", &profile.header.version.to_string()));
    println!("{}", kv("overall", profile.overall_confidence().label()));
    Ok(())
}


pub fn cmd_specialize(profile: Option<&str>, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let plan = plan_size_specialization(&info.topology, &SpecializeConfig::default());
    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        return Ok(());
    }
    println!("{}", header("silicera specialize"));
    for n in &plan.notes {
        println!("  {n}");
    }
    println!();
    println!("  decision tree thresholds:");
    println!("{}", kv("L1D", &format_bytes(plan.tree.l1_bytes)));
    println!("{}", kv("L2", &format_bytes(plan.tree.l2_bytes)));
    println!("{}", kv("L3", &format_bytes(plan.tree.l3_bytes)));
    println!("{}", kv("fallback", &plan.tree.fallback_variant));
    if let Some(p) = profile {
        let h = HnepProfile::read_from(Path::new(p))?;
        println!();
        println!("{}", kv("profile", p));
        println!("{}", kv("workloads", &h.workloads.len().to_string()));
        println!("{}", kv("size_classes", &h.size_classes.len().to_string()));
        if let Some(tree) = &h.decision_tree {
            println!("  profile tree winners by size probe:");
            for sz in [1024u64, 100_000, 2_000_000, 64_000_000] {
                println!("    size={sz} → {}", tree.evaluate(sz));
            }
        }
    }
    Ok(())
}


pub fn cmd_verify(profile: &str, strict_machine: bool, spot_check: bool) -> Result<()> {
    let policy = if strict_machine {
        MismatchPolicy::StrictMachine
    } else {
        MismatchPolicy::FallbackBaseline
    };
    println!("{}", header("silicera verify"));
    let d = Dispatcher::open(Path::new(profile), policy)?;
    println!("{}", kv("profile", profile));
    println!("  {}", d.status().replace('\n', "\n  "));
    println!("{}", kv("baseline_only", &d.using_baseline().to_string()));
    let p = HnepProfile::read_from(Path::new(profile))?;
    p.verify_integrity()?;
    println!("{}", kv("integrity", "OK"));
    println!("{}", kv("digest", &p.digest.hex));
    println!("{}", kv("hnep_version", &p.header.version.to_string()));

    let live = EnvironmentSnapshot::capture();
    let live_fp = detect_hardware()?
        .fingerprint
        .as_ref()
        .map(|f| f.value.clone());
    let stale = assess_staleness(
        &p,
        &live,
        live_fp.as_deref(),
        &StalenessPolicy::default(),
    );
    println!("{}", kv("staleness", stale.severity.label()));
    for s in &stale.signals {
        println!(
            "  · drift {} [{}]: {} → {} ({})",
            s.field,
            s.severity.label(),
            s.trained,
            s.live,
            s.note
        );
    }
    if stale.retrain.recommended {
        println!("{}", kv("retrain", &stale.retrain.summary));
        if !stale.retrain.suggested_command.is_empty() {
            println!("{}", kv("suggest", &stale.retrain.suggested_command));
        }
        let plan = plan_partial_retrain(&p, &stale.signals);
        println!("{}", kv("partial_plan", &plan.summary));
        if plan.is_partial && !plan.only_flag.is_empty() {
            println!("{}", kv("only", &plan.only_flag));
        }
        if stale.severity == DriftSeverity::Hard && !d.using_baseline() {
            println!("  note: hard drift with matching fingerprint — refresh before trusting winners");
        }
    } else {
        println!("{}", kv("retrain", "not recommended (env within policy)"));
    }

    if spot_check && !d.using_baseline() {
        let info = detect_hardware()?;
        if info.support.is_supported() {
            println!();
            println!("  spot-check (tiny sample; explicit only — not a background daemon):");
            let report = spot_check_profile(
                &info,
                &p,
                4,
                &MeasurementConfig {
                    warmup: 2,
                    iterations: 8,
                    ..Default::default()
                },
            )?;
            for item in &report.items {
                let tag = if item.ok { "OK" } else { "CHECK" };
                println!(
                    "  · [{tag}] {} winner={} — {}",
                    item.workload, item.profile_winner, item.observation
                );
            }
            println!("  {}", report.summary);
        }
    }
    Ok(())
}


pub fn cmd_explain(profile: Option<&str>, size: Option<u64>, workload: Option<&str>) -> Result<()> {
    println!("{}", header("silicera explain"));
    let info = detect_hardware()?;
    let plan = plan_size_specialization(&info.topology, &SpecializeConfig::default());
    let loaded = profile
        .map(|path| HnepProfile::read_from(Path::new(path)))
        .transpose()?;
    let tree = loaded
        .as_ref()
        .and_then(|p| p.decision_tree.as_ref())
        .unwrap_or(&plan.tree);
    if let Some(sz) = size {
        println!("{}", kv("size_bytes", &sz.to_string()));
        if loaded.as_ref().and_then(|p| p.decision_tree.as_ref()).is_some() {
            println!("  (walking profile decision tree)");
        } else {
            println!("  (walking topology plan tree)");
        }
        for step in tree.explain(sz) {
            println!("  → {step}");
        }
    } else {
        println!("  pass --size <bytes> to walk the decision tree");
    }
    if let Some(path) = profile {
        let p = loaded.as_ref().expect("profile loaded");
        println!();
        println!("{}", kv("profile", path));
        println!("{}", kv("hnep_version", &p.header.version.to_string()));
        if let Some(wname) = workload {
            if let Some(w) = p.workloads.iter().find(|w| w.name == wname) {
                println!("{}", kv("workload", &w.name));
                println!("{}", kv("winner", &w.winner));
                println!("{}", kv("confidence", w.confidence.label()));
                println!("  {}", w.rationale);
            } else if let Some(sc) = p.size_class_winner(wname) {
                println!("{}", kv("size_class", &sc.class));
                println!("{}", kv("winner", &sc.winner));
                println!("{}", kv("confidence", sc.confidence.label()));
                println!("  {}", sc.rationale);
            } else {
                bail!("workload/size-class '{wname}' not in profile");
            }
        } else {
            for w in &p.workloads {
                println!(
                    "  · {} → {} [{}]",
                    w.name,
                    w.winner,
                    w.confidence.label()
                );
            }
            for sc in &p.size_classes {
                println!(
                    "  · size_class {} → {} [{}] ws={}",
                    sc.class,
                    sc.winner,
                    sc.confidence.label(),
                    format_bytes(sc.working_set_bytes)
                );
            }
        }
        if let Some(tree) = &p.decision_tree {
            let report = measure_dispatch_overhead_report(
                tree,
                &MeasurementConfig {
                    warmup: 5,
                    iterations: 100,
                    ..Default::default()
                },
            )?;
            println!();
            println!(
                "  dispatch_overhead batch direct={:.0} ns tree={:.0} ns delta={:.0} ns",
                report.batch_direct.median_ns,
                report.batch_tree.median_ns,
                report.batch_delta_ns
            );
            println!(
                "  dispatch_overhead per-size delta median={:.0} mean={:.0} min={:.0} max={:.0} ns",
                report.delta_median_ns,
                report.delta_mean_ns,
                report.delta_min_ns,
                report.delta_max_ns
            );
        }
    }
    Ok(())
}


pub fn cmd_profile(path: &str, json: bool) -> Result<()> {
    let p = HnepProfile::read_from(Path::new(path))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&p)?);
        return Ok(());
    }
    println!("{}", header("silicera profile"));
    println!("{}", kv("path", path));
    println!("{}", kv("format", &p.header.format));
    println!("{}", kv("version", &p.header.version.to_string()));
    println!("{}", kv("created", &p.header.created_at));
    println!("{}", kv("fingerprint", &p.header.fingerprint));
    println!("{}", kv("label", &p.header.label));
    println!("{}", kv("digest", &format!("{}:{}", p.digest.alg, p.digest.hex)));
    println!("{}", kv("confidence", p.overall_confidence().label()));
    println!();
    for w in &p.workloads {
        println!(
            "  · {:<16} winner={:<12} {}",
            w.name,
            w.winner,
            w.confidence.label()
        );
    }
    for sc in &p.size_classes {
        println!(
            "  · sc:{:<13} winner={:<12} {}",
            sc.class,
            sc.winner,
            sc.confidence.label()
        );
    }
    Ok(())
}


pub fn cmd_validate(profile: &str, json: bool) -> Result<()> {
    let p = HnepProfile::read_from(Path::new(profile))?;
    p.verify_integrity()?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "path": profile,
                "integrity": "OK",
                "digest": p.digest.hex,
                "version": p.header.version,
                "format": p.header.format,
                "workloads": p.workloads.len(),
                "size_classes": p.size_classes.len(),
            })
        );
        return Ok(());
    }
    println!("{}", header("silicera validate"));
    println!("{}", kv("path", profile));
    println!("{}", kv("integrity", "OK"));
    println!("{}", kv("digest", &p.digest.hex));
    println!("{}", kv("version", &p.header.version.to_string()));
    println!("{}", kv("workloads", &p.workloads.len().to_string()));
    println!("{}", kv("size_classes", &p.size_classes.len().to_string()));
    Ok(())
}


pub fn cmd_digest(profile: &str, json: bool) -> Result<()> {
    let p = HnepProfile::read_from(Path::new(profile))?;
    let ok = p.verify_integrity().is_ok();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "path": profile,
                "alg": p.digest.alg,
                "hex": p.digest.hex,
                "verified": ok,
            })
        );
        return Ok(());
    }
    println!("{}", header("silicera digest"));
    println!("{}", kv("path", profile));
    println!("{}", kv("alg", &p.digest.alg));
    println!("{}", kv("hex", &p.digest.hex));
    println!("{}", kv("verified", &tag(ok)));
    if !ok {
        bail!("integrity verification failed");
    }
    Ok(())
}


pub fn cmd_tree(profile: Option<&str>, json: bool) -> Result<()> {
    let tree = if let Some(path) = profile {
        let p = HnepProfile::read_from(Path::new(path))?;
        p.decision_tree
            .clone()
            .context("profile has no decision tree")?
    } else {
        let info = detect_hardware()?;
        if !info.support.is_supported() {
            eprintln!("{}", info.support.message());
            std::process::exit(2);
        }
        plan_size_specialization(&info.topology, &SpecializeConfig::default()).tree
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&tree)?);
        return Ok(());
    }
    println!("{}", header("silicera tree"));
    println!("{}", kv("L1D", &format_bytes(tree.l1_bytes)));
    println!("{}", kv("L2", &format_bytes(tree.l2_bytes)));
    println!("{}", kv("L3", &format_bytes(tree.l3_bytes)));
    println!("{}", kv("fallback", &tree.fallback_variant));
    println!();
    dump_node(&tree.root, 0);
    Ok(())
}


pub fn cmd_dispatch(
    profile: &str,
    workload: Option<&str>,
    size: Option<u64>,
    strict_machine: bool,
    json: bool,
) -> Result<()> {
    let policy = if strict_machine {
        MismatchPolicy::StrictMachine
    } else {
        MismatchPolicy::FallbackBaseline
    };
    let d = Dispatcher::open(Path::new(profile), policy)?;
    let by_workload = workload.map(|w| (w.to_string(), d.workload(w).to_string()));
    let by_size = size.map(|sz| (sz, d.size(sz).to_string()));
    if by_workload.is_none() && by_size.is_none() {
        bail!("pass --workload <name> and/or --size <bytes>");
    }
    if json {
        println!(
            "{}",
            serde_json::json!({
                "profile": profile,
                "status": d.status(),
                "baseline_only": d.using_baseline(),
                "workload": by_workload.as_ref().map(|(n, v)| serde_json::json!({"name": n, "variant": v})),
                "size": by_size.as_ref().map(|(sz, v)| serde_json::json!({"bytes": sz, "variant": v})),
            })
        );
        return Ok(());
    }
    println!("{}", header("silicera dispatch"));
    println!("{}", kv("profile", profile));
    println!("  {}", d.status().replace('\n', "\n  "));
    println!("{}", kv("baseline_only", &d.using_baseline().to_string()));
    if let Some((n, v)) = &by_workload {
        println!("{}", kv("workload", n));
        println!("{}", kv("variant", v));
    }
    if let Some((sz, v)) = &by_size {
        println!("{}", kv("size_bytes", &sz.to_string()));
        println!("{}", kv("variant", v));
    }
    Ok(())
}


pub fn cmd_staleness(profile: &str, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    let p = HnepProfile::read_from(Path::new(profile))?;
    let live = EnvironmentSnapshot::capture();
    let report = assess_staleness(
        &p,
        &live,
        info.fingerprint.as_ref().map(|f| f.value.as_str()),
        &StalenessPolicy::default(),
    );
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera staleness"));
    println!("{}", kv("profile", profile));
    println!("{}", kv("severity", report.severity.label()));
    for s in &report.signals {
        println!(
            "  · {} [{}]: {} → {} ({})",
            s.field,
            s.severity.label(),
            s.trained,
            s.live,
            s.note
        );
    }
    if report.retrain.recommended {
        println!("{}", kv("retrain", &report.retrain.summary));
        if !report.retrain.suggested_command.is_empty() {
            println!("{}", kv("suggest", &report.retrain.suggested_command));
        }
    } else {
        println!("{}", kv("retrain", "not recommended"));
    }
    Ok(())
}


pub fn cmd_retrain(profile: &str, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    let p = HnepProfile::read_from(Path::new(profile))?;
    let live = EnvironmentSnapshot::capture();
    let stale = assess_staleness(
        &p,
        &live,
        info.fingerprint.as_ref().map(|f| f.value.as_str()),
        &StalenessPolicy::default(),
    );
    let plan = plan_partial_retrain(&p, &stale.signals);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "staleness": stale,
                "plan": plan,
            })
        );
        return Ok(());
    }
    println!("{}", header("silicera retrain"));
    println!("{}", kv("profile", profile));
    println!("{}", kv("severity", stale.severity.label()));
    println!("{}", kv("summary", &plan.summary));
    println!("{}", kv("partial", &plan.is_partial.to_string()));
    if !plan.only_flag.is_empty() {
        println!("{}", kv("only", &plan.only_flag));
    }
    if !plan.must_retrain.is_empty() {
        println!("  must_retrain:");
        for t in &plan.must_retrain {
            println!("    · {t}");
        }
    }
    if !plan.keep.is_empty() {
        println!("  keep:");
        for t in &plan.keep {
            println!("    · {t}");
        }
    }
    if !stale.retrain.suggested_command.is_empty() {
        println!("{}", kv("suggest", &stale.retrain.suggested_command));
    }
    Ok(())
}

/// Print composite profile health (integrity + confidence + staleness).
pub fn cmd_health(profile: &str, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    let p = HnepProfile::read_from(Path::new(profile))?;
    let live = EnvironmentSnapshot::capture();
    let health = ProfileHealth::assess_default(
        &p,
        &live,
        info.fingerprint.as_ref().map(|f| f.value.as_str()),
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&health)?);
        return Ok(());
    }
    println!("{}", header("silicera health"));
    println!("{}", kv("profile", profile));
    println!("{}", kv("label", &health.profile_label));
    println!("{}", kv("fingerprint", &health.fingerprint));
    println!("{}", kv("score", &format!("{}/100", health.score)));
    println!("{}", kv("grade", health.grade.label()));
    println!(
        "{}",
        kv(
            "integrity",
            if health.integrity_ok { "ok" } else { "failed" }
        )
    );
    println!("{}", kv("confidence", &health.confidence));
    println!("{}", kv("staleness", &health.staleness));
    println!(
        "{}",
        kv(
            "retrain",
            if health.retrain_recommended {
                "recommended"
            } else {
                "not needed"
            }
        )
    );
    for n in &health.notes {
        println!("  · {n}");
    }
    if health.retrain_plan.is_partial && !health.retrain_plan.only_flag.is_empty() {
        println!("{}", kv("partial", &health.retrain_plan.only_flag));
    }
    Ok(())
}
