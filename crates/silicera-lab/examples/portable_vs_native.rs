//! Portable vs Native vs Silicera — identical knobs; report wins and losses.
//!
//! ```bash
//! cargo run -p silicera-lab --example portable_vs_native
//! ```
//!
//! CLI: `cargo run -p silicera-cli -- harness --domain all`

use silicera::hardware::detect_hardware;
use silicera_lab::arms::ArmsHarnessConfig;

fn main() -> silicera::Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("unsupported host: {}", info.support.message());
        std::process::exit(2);
    }

    let cfg = ArmsHarnessConfig {
        warmup: 5,
        iterations: 30,
        min_improvement: 0.03,
        mem_target: "L2".into(),
        mem_all_sizes: true,
    };
    let suite = silicera_lab::arms::run_domain_harness(&info, &cfg, "all")?;
    let reports = suite.reports;

    println!("=== Portable vs Native vs Silicera ===");
    println!("host         {}", info.brand);
    if let Some(fp) = &info.fingerprint {
        println!("fingerprint  {}", fp.value);
    }
    println!(
        "knobs        warmup={} iterations={} min_improvement={}",
        cfg.warmup, cfg.iterations, cfg.min_improvement
    );
    println!();

    for r in &reports {
        println!("-- {} --", r.workload);
        for a in &r.arms {
            println!(
                "  {:<28} median={:>10.0} ns  n={}  {}",
                a.arm,
                a.summary.median_ns,
                a.summary.n,
                a.summary.stability.label()
            );
        }
        println!("  fastest_arm           {}", r.fastest_arm);
        if r.native_beats_silicera {
            println!("  native_beats_silicera YES (print the loss)");
        }
        if r.portable_wins {
            println!("  portable_wins         YES");
        }
        println!("  {}", r.conclusion);
        println!();
    }

    println!("Methodology: docs/benchmarks/portable-vs-native.md");
    println!("Phase I native arm is a stand-in kernel, not LLVM -march=native codegen.");
    Ok(())
}

