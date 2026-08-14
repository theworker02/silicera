//! Silicon Split — Machine A train + Machine B placeholder (multi-machine protocol).
//!
//! ```bash
//! cargo run -p silicera-lab --example silicon_split
//! ```
//!
//! CLI:
//! ```bash
//! cargo run -p silicera-cli -- silicon-split train --role A -o out/machine_a.hnep
//! cargo run -p silicera-cli -- silicon-split placeholder --profile out/machine_a.hnep
//! cargo run -p silicera-cli -- silicon-split report --a out/machine_a.hnep
//! ```

use std::path::Path;

use silicera::hardware::detect_hardware;
use silicera::measure::MeasurementConfig;
use silicera_lab::experiments::{run_experiment, ExperimentId};
use silicera_lab::silicon_split::{
    sanitize_export, train_split_profile, write_export, write_machine_b_placeholder,
    SiliconSplitProtocol,
};
use silicera::compare::MeasurementEcho;

fn main() -> silicera::Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("unsupported host: {}", info.support.message());
        eprintln!("tip: cargo run -p silicera-cli -- inspect --mock zen5");
        std::process::exit(2);
    }

    let cfg = MeasurementConfig {
        warmup: 3,
        iterations: 20,
        ..Default::default()
    };
    let report = run_experiment(ExperimentId::SiliconSplit, &info, cfg)?;

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

    // Produce publishable Machine A artifacts + Machine B placeholder schema.
    let protocol = SiliconSplitProtocol {
        measurement: MeasurementEcho {
            warmup: 3,
            iterations: 20,
            min_improvement: 0.03,
            ..MeasurementEcho::default()
        },
        ..Default::default()
    };
    let a_path = Path::new("out/machine_a.hnep");
    let (profile, export) = train_split_profile(&info, "A", a_path, &protocol)?;
    let export_path = Path::new("out/machine_a.split.json");
    write_export(&export, export_path)?;
    // Re-sanitize with protocol echo (train_split_profile already did).
    let _ = sanitize_export(&profile, "A", &protocol.measurement);
    let ph_path = Path::new("out/machine_b.placeholder.json");
    let ph = write_machine_b_placeholder(&profile, ph_path)?;

    println!("artifacts");
    println!("  Machine A HNEP     {}", a_path.display());
    println!("  Machine A export   {}", export_path.display());
    println!("  Machine B schema   {}", ph_path.display());
    println!("  strategy_vector_A  {}", export.strategy_vector.compact());
    println!();
    println!("verdict              UNKNOWN (Machine B placeholder)");
    println!("next                 {}", ph.instructions.first().cloned().unwrap_or_default());
    println!();
    println!("compare when B exists:");
    println!("  silicera compare out/machine_a.hnep out/machine_b.hnep");
    Ok(())
}

