//! Measurement, harness, alignment, and native-artifact commands.

use std::path::Path;

use anyhow::Result;
use silicera::hardware::detect_hardware;
use silicera::measure::{MeasurementConfig, MeasurementEngine};
use silicera_lab::arms::{run_domain_harness, ArmsHarnessConfig};
use silicera_lab::bench::{
    BranchBench, CacheTarget, ConcurrencyBench, FloatBench, IntegerBench, MemoryBench,
};
use silicera_lab::calm::run_calm_check;
use silicera_lab::measure_alignment;
use silicera_lab::measure_placement;
use silicera_lab::native_artifacts::{
    default_artifact_kernels, find_workspace_root, run_native_artifact_compare,
    run_native_artifact_suite,
};
use silicera_lab::single_machine::merge_artifacts_into_profile;
use silicera_lab::threads::measure_thread_count_sweep;

use crate::style::{header, kv};

pub fn cmd_benchmark(domain: &str, iterations: usize) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let eng = MeasurementEngine::new(MeasurementConfig {
        warmup: 3,
        iterations,
        ..Default::default()
    });
    println!("{}", header("silicera benchmark"));
    println!("{}", kv("host", &info.brand));
    println!("{}", kv("iterations", &iterations.to_string()));
    println!();

    let run_mem = domain == "all" || domain == "memory";
    let run_int = domain == "all" || domain == "integer";
    let run_float = domain == "all" || domain == "float";
    let run_branch = domain == "all" || domain == "branch";
    let run_conc = domain == "all" || domain == "concurrency";

    if run_mem {
        for t in [CacheTarget::L1, CacheTarget::L2, CacheTarget::L3, CacheTarget::Dram] {
            let b = MemoryBench::for_target(&info.topology, t);
            let s = eng.measure(|| {
                let _ = b.run();
            })?;
            println!(
                "  memory {:<28} median={:>10.0} ns  {}  flags={:?}",
                t.describe(&info.topology),
                s.median_ns,
                s.stability.label(),
                s.flags
            );
        }
    }
    if run_int {
        let b = IntegerBench { n: 1 };
        let s = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        println!(
            "  integer lcg                     median={:>10.0} ns  {}",
            s.median_ns,
            s.stability.label()
        );
    }
    if run_float {
        let b = FloatBench::new(4096);
        let s = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        println!(
            "  float sum                       median={:>10.0} ns  {}",
            s.median_ns,
            s.stability.label()
        );
    }
    if run_branch {
        let b = BranchBench::new(8192);
        let s = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        println!(
            "  branch mix                      median={:>10.0} ns  {}",
            s.median_ns,
            s.stability.label()
        );
    }
    if run_conc {
        let b = ConcurrencyBench {
            iters: 20_000,
            threads: 4.min(info.topology.thread_count().max(1)),
        };
        let s = eng.measure(|| {
            let _ = b.run_baseline();
        })?;
        println!(
            "  concurrency atomics             median={:>10.0} ns  {}",
            s.median_ns,
            s.stability.label()
        );
    }
    println!();
    println!("  values are measured on this host; do not copy them as marketing claims.");
    Ok(())
}


pub fn cmd_harness(domain: &str, iterations: usize, warmup: usize, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let cfg = ArmsHarnessConfig {
        warmup,
        iterations,
        min_improvement: 0.03,
        mem_target: "L2".into(),
        mem_all_sizes: domain == "all" || domain == "memory",
    };
    let suite = run_domain_harness(&info, &cfg, domain)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&suite)?);
        return Ok(());
    }
    println!("{}", header("silicera harness"));
    println!("{}", kv("host", &info.brand));
    println!(
        "{}",
        kv(
            "knobs",
            &format!(
                "warmup={} iterations={} min_improvement={}",
                cfg.warmup, cfg.iterations, cfg.min_improvement
            )
        )
    );
    for r in &suite.reports {
        println!();
        println!("  {}", r.workload);
        for a in &r.arms {
            println!(
                "    {:<28} median={:>10.0} ns  {}",
                a.arm,
                a.summary.median_ns,
                a.summary.stability.label()
            );
        }
        println!("    fastest_arm={}", r.fastest_arm);
        if r.native_beats_silicera {
            println!("    NATIVE BEATS SILICERA (loss — do not hide)");
        }
        if r.portable_wins {
            println!("    PORTABLE WINS (no specialization gain under these knobs)");
        }
        println!("    {}", r.conclusion);
    }
    println!();
    println!(
        "  suite: portable_wins={} native_beats_silicera={} reports={}",
        suite.portable_win_count, suite.native_beats_silicera_count, suite.reports.len()
    );
    for line in &suite.size_winner_changes {
        println!("  size-class: {line}");
    }
    println!();
    println!("  methodology: docs/benchmarks/portable-vs-native.md");
    Ok(())
}


pub fn cmd_threads(iterations: usize, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let report = measure_thread_count_sweep(
        &info,
        &MeasurementConfig {
            warmup: 3,
            iterations,
            ..Default::default()
        },
        80_000,
        None,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera threads"));
    println!("{}", kv("host", &report.host_brand));
    println!("{}", kv("logical_cpus", &report.logical_cpus.to_string()));
    println!();
    for p in &report.points {
        println!(
            "  threads={:<3} baseline={:>10.0} ns  candidate={:>10.0} ns  faster={}",
            p.threads,
            p.baseline.median_ns,
            p.candidate.median_ns,
            p.faster
        );
    }
    println!();
    println!("  {}", report.conclusion);
    println!("  {}", report.note);
    Ok(())
}


pub fn cmd_align(len: usize, iterations: usize, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let report = measure_alignment(
        &info,
        len,
        &MeasurementConfig {
            warmup: 3,
            iterations,
            ..Default::default()
        },
        0.03,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera align"));
    println!("{}", kv("host", &report.host_brand));
    println!("{}", kv("len", &report.len.to_string()));
    println!("{}", kv("alignment_matters", &report.alignment_matters.to_string()));
    for s in &report.samples {
        println!(
            "  · offset={:<2} scalar={:.0} chunked={:.0} winner={} imp={:.1}%",
            s.offset,
            s.scalar.median_ns,
            s.chunked.median_ns,
            s.winner,
            s.improvement * 100.0
        );
    }
    println!("  {}", report.recommendation);
    Ok(())
}


pub fn cmd_placement(threads: usize, iterations: usize, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let report = measure_placement(
        &info,
        threads,
        50_000,
        &MeasurementConfig {
            warmup: 2,
            iterations,
            ..Default::default()
        },
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera placement"));
    println!("{}", kv("host", &report.host_brand));
    println!("{}", kv("logical_cpus", &report.logical_cpus.to_string()));
    println!("{}", kv("fastest", &report.fastest));
    for s in &report.samples {
        println!(
            "  · {:<12} median={:.0} ns affinity={} — {}",
            s.strategy, s.summary.median_ns, s.affinity_applied, s.notes
        );
    }
    println!("  {}", report.conclusion);
    Ok(())
}


pub fn cmd_native_artifacts(
    kernel: &str,
    iterations: usize,
    json: bool,
    workspace: Option<&str>,
    suite: bool,
    merge_profile: Option<&str>,
) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let root = if let Some(w) = workspace {
        Path::new(w).to_path_buf()
    } else {
        find_workspace_root(Path::new("."))
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| find_workspace_root(p.parent()?))
            })
            .ok_or_else(|| {
                anyhow::anyhow!("could not locate workspace root (benchmarks/arm_kernels)")
            })?
    };
    println!("{}", header("silicera native-artifacts"));
    println!("{}", kv("workspace", &root.display().to_string()));

    if suite {
        println!("{}", kv("mode", "suite (all kernels)"));
        println!("  building portable + native for each kernel…");
        let suite_report =
            run_native_artifact_suite(&info, default_artifact_kernels(), iterations, 0.03, &root)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&suite_report)?);
        } else {
            println!("{}", kv("host", &suite_report.host_brand));
            for r in &suite_report.reports {
                println!(
                    "  · {:<12} portable={:.0} native={:.0} → {}",
                    r.kernel, r.portable.median_ns, r.native.median_ns, r.silicera_choice
                );
            }
            println!("  {}", suite_report.summary);
        }
        if let Some(path) = merge_profile {
            merge_artifacts_into_profile(Path::new(path), &suite_report)?;
            println!("{}", kv("merged_into", path));
        }
        return Ok(());
    }

    println!("{}", kv("kernel", kernel));
    println!("  building portable (x86-64-v2) and native (-C target-cpu=native)…");
    let report = run_native_artifact_compare(&info, kernel, iterations, 0.03, &root)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", kv("host", &report.host_brand));
    println!(
        "  · portable  median={:.0} ns  {}",
        report.portable.median_ns, report.portable.notes
    );
    println!(
        "  · native    median={:.0} ns  {}",
        report.native.median_ns, report.native.notes
    );
    println!("{}", kv("choice", &report.silicera_choice));
    println!("  {}", report.conclusion);
    if let Some(path) = merge_profile {
        let suite_one = run_native_artifact_suite(&info, &[kernel], iterations, 0.03, &root)?;
        merge_artifacts_into_profile(Path::new(path), &suite_one)?;
        println!("{}", kv("merged_into", path));
    }
    Ok(())
}


pub fn cmd_calm_check(iterations: usize, json: bool) -> Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("{}", info.support.message());
        std::process::exit(2);
    }
    let report = run_calm_check(&info, iterations)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("{}", header("silicera calm-check"));
    println!("{}", kv("host", &report.host_brand));
    println!("{}", kv("pass_a", &format!("{} @ {:.0} ns", report.pass_a, report.median_a_ns)));
    println!("{}", kv("pass_b", &format!("{} @ {:.0} ns", report.pass_b, report.median_b_ns)));
    println!("{}", kv("relative_drift", &format!("{:.1}%", report.relative_drift * 100.0)));
    println!("{}", kv("calm", &report.calm.to_string()));
    println!("  {}", report.recommendation);
    Ok(())
}
