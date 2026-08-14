//! Cold Machine — first-run path with no HNEP (baseline only).
//!
//! ```bash
//! cargo run -p silicera-lab --example cold_machine
//! ```
//!
//! Train a profile when ready:
//! `cargo run -p silicera-cli -- train -o out/profile.hnep`

use silicera::hardware::detect_hardware;
use silicera::measure::MeasurementConfig;
use silicera_lab::experiments::{run_experiment, ExperimentId};

fn main() -> silicera::Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("unsupported host: {}", info.support.message());
        std::process::exit(2);
    }

    let cfg = MeasurementConfig {
        warmup: 2,
        iterations: 15,
        ..Default::default()
    };
    let report = run_experiment(ExperimentId::ColdMachine, &info, cfg)?;

    println!("=== {} ===", report.title);
    println!("host         {}", report.host_brand);
    if let Some(fp) = &report.fingerprint {
        println!("fingerprint  {fp}");
    }
    println!();
    for arm in &report.arms {
        print!("  {:<22}", arm.name);
        if let Some(s) = &arm.summary {
            println!(
                "  median={:.0} ns  n={}  {}",
                s.median_ns,
                s.n,
                s.stability.label()
            );
        } else {
            println!("  (not measured)");
        }
        if !arm.notes.is_empty() {
            println!("    notes: {}", arm.notes);
        }
    }
    println!();
    println!("{}", report.conclusion);
    println!();
    println!("next: cargo run -p silicera-cli -- train -o out/profile.hnep");
    Ok(())
}
