//! Compare, silicon-split, lab, eval, and experiment commands.

use std::path::Path;

use anyhow::{bail, Context, Result};
use silicera::arch_compare::{compare_architectures, compare_mock_zen4_zen5, side_from_info};
use silicera::compare::{compare_profiles, compare_with_placeholder, MeasurementEcho};
use silicera::hardware::{detect_hardware, HardwareBackend, MockHardware};
use silicera::hnep::HnepProfile;
use silicera::knowledge::KnowledgePack;
use silicera::measure::MeasurementConfig;
use silicera::topology::format_bytes;
use silicera_lab::experiments::{run_experiment, ExperimentId};
use silicera_lab::native_artifacts::find_workspace_root;
use silicera_lab::silicon_split::{
    sanitize_export, split_report_paths, train_split_profile, write_export,
    write_machine_b_placeholder, SiliconSplitProtocol,
};
use silicera_lab::single_machine::{
    merge_artifacts_into_profile, run_single_machine_eval, write_eval_outputs,
};

use super::common::load_profile_or_export;
use crate::style::{header, kv, tag};

pub fn cmd_compare(a: &str, b: Option<&str>, json: bool, placeholder: bool) -> Result<()> {
    println!("{}", header("silicera compare"));
    let report = if placeholder || b.is_none() {
        let pa = load_profile_or_export(a)?;
        compare_with_placeholder(&pa, "second-zen-box")
    } else {
        let b = b.unwrap();
        let pa = load_profile_or_export(a)?;
        let pb = load_profile_or_export(b)?;
        compare_profiles(&pa, &pb)
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("{}", kv("protocol", &report.protocol));
    println!("{}", kv("fp_a", &report.fingerprint_a));
    println!("{}", kv("fp_b", &report.fingerprint_b));
    println!("{}", kv("fp_equal", &tag(report.fingerprints_equal)));
    println!("{}", kv("b_placeholder", &tag(report.b_is_placeholder)));
    println!();
    println!("  strategy A: {}", report.strategy_a.compact());
    println!("  strategy B: {}", report.strategy_b.compact());
    println!();
    println!(
        "  {:<8} {:<18} {:<12} {:<12} {:<10} {}",
        "kind", "name", "winner_a", "winner_b", "delta", "status"
    );
    for t in &report.targets {
        let delta = t
            .median_rel_delta
            .map(|d| format!("{:+.1}%", d * 100.0))
            .unwrap_or_else(|| "—".into());
        println!(
            "  {:<8} {:<18} {:<12} {:<12} {:<10} {}  {}",
            t.kind,
            t.name,
            t.winner_a.as_deref().unwrap_or("—"),
            t.winner_b.as_deref().unwrap_or("—"),
            delta,
            t.divergence.label(),
            t.note
        );
    }
    println!();
    println!("{}", kv("diverged", &report.diverged_count.to_string()));
    println!("{}", kv("same", &report.same_count.to_string()));
    println!("{}", kv("verdict", report.verdict.label()));
    println!("  {}", report.summary);
    Ok(())
}


pub fn cmd_silicon_split(
    action: &str,
    role: &str,
    profile: Option<&str>,
    output: &str,
    a: Option<&str>,
    b: Option<&str>,
    iterations: usize,
    json: bool,
) -> Result<()> {
    match action {
        "train" => {
            let info = detect_hardware()?;
            if !info.support.is_supported() {
                eprintln!("{}", info.support.message());
                std::process::exit(2);
            }
            let protocol = SiliconSplitProtocol {
                measurement: MeasurementEcho {
                    warmup: 3,
                    iterations,
                    min_improvement: 0.03,
                    ..MeasurementEcho::default()
                },
                ..Default::default()
            };
            println!("{}", header("silicera silicon-split train"));
            println!("{}", kv("role", role));
            println!("{}", kv("host", &info.brand));
            println!("{}", kv("output", output));
            let (profile, export) = train_split_profile(&info, role, Path::new(output), &protocol)?;
            let export_path = Path::new(output).with_extension("split.json");
            write_export(&export, &export_path)?;
            if role.eq_ignore_ascii_case("A") {
                let ph = Path::new("out/machine_b.placeholder.json");
                write_machine_b_placeholder(&profile, ph)?;
                println!("{}", kv("placeholder", &ph.display().to_string()));
            }
            println!("{}", kv("export", &export_path.display().to_string()));
            println!("{}", kv("strategy", &export.strategy_vector.compact()));
            println!("{}", kv("digest", &profile.digest.hex));
            if role.eq_ignore_ascii_case("A") {
                println!();
                println!("  Machine B not measured — verdict UNKNOWN until a second Zen host trains.");
            }
            Ok(())
        }
        "export" => {
            let path = profile.context("--profile required for export")?;
            let p = HnepProfile::read_from(Path::new(path))?;
            let export = sanitize_export(
                &p,
                role,
                &MeasurementEcho {
                    warmup: 3,
                    iterations,
                    min_improvement: 0.03,
                    ..MeasurementEcho::default()
                },
            );
            write_export(&export, Path::new(output))?;
            println!("{}", header("silicera silicon-split export"));
            println!("{}", kv("wrote", output));
            Ok(())
        }
        "placeholder" => {
            let path = profile.context("--profile required for placeholder")?;
            let p = HnepProfile::read_from(Path::new(path))?;
            let out = if output.is_empty() {
                "out/machine_b.placeholder.json".to_string()
            } else {
                output.to_string()
            };
            let ph = write_machine_b_placeholder(&p, Path::new(&out))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&ph)?);
            } else {
                println!("{}", header("silicera silicon-split placeholder"));
                println!("{}", kv("wrote", &out));
                println!("{}", kv("schema", &ph.schema));
                println!("  instructions:");
                for i in &ph.instructions {
                    println!("    - {i}");
                }
                println!("  do_not:");
                for d in &ph.do_not {
                    println!("    - {d}");
                }
            }
            Ok(())
        }
        "report" => {
            let a_path = a.context("--a required for report")?;
            let report = split_report_paths(Path::new(a_path), b.map(Path::new))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", header("silicera silicon-split report"));
                println!("{}", kv("verdict", report.verdict.label()));
                println!("{}", kv("diverged", &report.diverged_count.to_string()));
                println!("{}", kv("same", &report.same_count.to_string()));
                println!("  strategy A: {}", report.strategy_a.compact());
                println!("  strategy B: {}", report.strategy_b.compact());
                for t in &report.targets {
                    println!(
                        "  · {} {}  A={} B={}  {}",
                        t.kind,
                        t.name,
                        t.winner_a.as_deref().unwrap_or("—"),
                        t.winner_b.as_deref().unwrap_or("—"),
                        t.divergence.label()
                    );
                }
                println!("  {}", report.summary);
            }
            Ok(())
        }
        other => bail!("unknown silicon-split action '{other}' (train|export|placeholder|report)"),
    }
}


pub fn cmd_lab() -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    silicera_lab::run_lab_ui(&info)?;
    Ok(())
}


pub fn cmd_arch_compare(json: bool, mock: bool) -> Result<()> {
    let packs = KnowledgePack::builtin();
    let report = if mock {
        compare_mock_zen4_zen5(&packs)?
    } else {
        let live = detect_hardware()?;
        if !live.support.is_supported() {
            eprintln!("{}", live.support.message());
            std::process::exit(2);
        }
        let zen4 = MockHardware::zen4_single_ccd().discover(&packs)?;
        compare_architectures(side_from_info(&live, &packs)?, side_from_info(&zen4, &packs)?)
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera arch-compare"));
    println!(
        "  A: {} {} domains={} cores={} L3={}",
        report.a.microarch,
        report.a.brand,
        report.a.domains,
        report.a.cores,
        format_bytes(report.a.l3_bytes)
    );
    println!(
        "  B: {} {} domains={} cores={} L3={}",
        report.b.microarch,
        report.b.brand,
        report.b.domains,
        report.b.cores,
        format_bytes(report.b.l3_bytes)
    );
    for d in &report.differences {
        println!("  · {d}");
    }
    for n in &report.notes {
        println!("  note: {n}");
    }
    println!("  {}", report.research_status);
    Ok(())
}


pub fn cmd_eval(
    output: &str,
    harness_iters: usize,
    artifact_iters: usize,
    workspace: Option<&str>,
    merge_profile: Option<&str>,
    json: bool,
) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let root = if let Some(w) = workspace {
        Path::new(w).to_path_buf()
    } else {
        find_workspace_root(Path::new(".")).ok_or_else(|| {
            anyhow::anyhow!("could not locate workspace root (benchmarks/arm_kernels)")
        })?
    };
    println!("{}", header("silicera eval"));
    println!("  Single-Zen evaluation pack (no second machine required).");
    println!("{}", kv("workspace", &root.display().to_string()));
    println!("  running calm-check → harness → artifacts → align → placement → threads…");
    let eval = run_single_machine_eval(&info, &root, harness_iters, artifact_iters)?;
    write_eval_outputs(&info, &eval, Path::new(output))?;
    if let Some(path) = merge_profile {
        merge_artifacts_into_profile(Path::new(path), &eval.artifacts)?;
        println!("{}", kv("merged_artifacts_into", path));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&eval)?);
    } else {
        println!("{}", kv("calm", &eval.calm.calm.to_string()));
        println!(
            "{}",
            kv(
                "harness",
                &format!(
                    "portable_wins={} native_beats_silicera={}",
                    eval.harness.portable_win_count, eval.harness.native_beats_silicera_count
                )
            )
        );
        println!("{}", kv("artifacts", &eval.artifacts.summary));
        println!("{}", kv("alignment_matters", &eval.alignment_matters.to_string()));
        println!("{}", kv("placement", &eval.placement_fastest));
        println!("{}", kv("best_threads", &eval.best_thread_count.to_string()));
        println!("{}", kv("wrote", output));
        println!("{}", kv("repro", &Path::new(output).with_extension("repro.json").display().to_string()));
        for d in &eval.deferred {
            println!("  deferred: {d}");
        }
        println!("  {}", eval.summary);
    }
    Ok(())
}


pub fn cmd_experiment(id: Option<&str>, list: bool, iterations: usize, json: bool) -> Result<()> {
    if list || id.is_none() {
        if json {
            let rows: Vec<_> = ExperimentId::all()
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "slug": e.slug(),
                        "title": e.title(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&rows)?);
        } else {
            println!("{}", header("silicera experiment list"));
            for e in ExperimentId::all() {
                println!("  · {:<36} {}", e.slug(), e.title());
            }
            println!();
            println!("  run: silicera research experiment <slug> --iterations {iterations}");
        }
        return Ok(());
    }
    let slug = id.unwrap();
    let eid = ExperimentId::all()
        .iter()
        .copied()
        .find(|e| e.slug() == slug)
        .with_context(|| format!("unknown experiment '{slug}'"))?;
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let report = run_experiment(
        eid,
        &info,
        MeasurementConfig {
            warmup: 3,
            iterations,
            ..Default::default()
        },
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera experiment"));
    println!("{}", kv("id", report.id.slug()));
    println!("{}", kv("title", &report.title));
    println!("{}", kv("host", &report.host_brand));
    if let Some(fp) = &report.fingerprint {
        println!("{}", kv("fingerprint", fp));
    }
    for a in &report.arms {
        if let Some(s) = &a.summary {
            println!(
                "  · {:<28} median={:.0} ns  — {}",
                a.name, s.median_ns, a.notes
            );
        } else {
            println!("  · {:<28} (not measured) — {}", a.name, a.notes);
        }
    }
    println!();
    println!("  {}", report.conclusion);
    Ok(())
}
