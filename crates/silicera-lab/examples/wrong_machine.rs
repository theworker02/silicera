//! Wrong Machine — foreign fingerprint must not apply specialized winners.
//!
//! ```bash
//! cargo run -p silicera-lab --example wrong_machine
//! ```
//!
//! Writes `out/foreign_demo.hnep` and loads it against the live host.
//! CLI check: `cargo run -p silicera-cli -- verify out/foreign_demo.hnep`

use std::path::Path;

use silicera::hardware::{detect_hardware, EnvironmentSnapshot};
use silicera::hnep::{
    Confidence, HnepHeader, HnepProfile, IntegrityDigest, SizeClassEntry, WorkloadEntry,
    HNEP_FORMAT, HNEP_VERSION,
};
use silicera::measure::MeasurementConfig;
use silicera::VERSION;
use silicera_lab::experiments::{run_experiment, ExperimentId};
use silicera_runtime::{Dispatcher, MismatchPolicy};

fn foreign_profile() -> silicera::Result<HnepProfile> {
    // Deliberately not this host — Zen4-shaped fingerprint with placeholder hashes.
    let header = HnepHeader {
        format: HNEP_FORMAT.into(),
        version: HNEP_VERSION,
        silicera_version: VERSION.into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        fingerprint: "SLC:AMD:ZEN4:19:61:00:deadbeefdeadbeef:cafebabecafebabe".into(),
        label: "foreign-demo-wrong-machine".into(),
    };
    let environment = EnvironmentSnapshot::capture();
    let workloads = vec![WorkloadEntry {
        name: "demo".into(),
        winner: "fast".into(),
        confidence: Confidence::High,
        rationale: "synthetic foreign profile for mismatch demo — not a measured win".into(),
        winner_median_ns: None,
        baseline_median_ns: None,
    }];
    let size_classes: Vec<SizeClassEntry> = Vec::new();
    let decision_tree = None;

    #[derive(serde::Serialize)]
    struct Payload<'a> {
        header: &'a HnepHeader,
        environment: &'a EnvironmentSnapshot,
        workloads: &'a [WorkloadEntry],
        size_classes: &'a [SizeClassEntry],
        decision_tree: &'a Option<silicera::specialize::DecisionTree>,
    }
    let payload = Payload {
        header: &header,
        environment: &environment,
        workloads: &workloads,
        size_classes: &size_classes,
        decision_tree: &decision_tree,
    };
    let digest = IntegrityDigest::sha256(&serde_json::to_vec(&payload)?);

    Ok(HnepProfile {
        header,
        environment,
        workloads,
        size_classes,
        decision_tree,
        digest,
    })
}

fn main() -> silicera::Result<()> {
    let info = detect_hardware()?;
    if !info.support.is_supported() {
        eprintln!("unsupported host: {}", info.support.message());
        std::process::exit(2);
    }

    let cfg = MeasurementConfig {
        warmup: 2,
        iterations: 12,
        ..Default::default()
    };
    let report = run_experiment(ExperimentId::WrongMachine, &info, cfg)?;

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
            println!("  (deferred — see foreign HNEP load below)");
        }
        if !arm.notes.is_empty() {
            println!("    notes: {}", arm.notes);
        }
    }
    println!();
    println!("{}", report.conclusion);
    println!();

    let out = Path::new("out/foreign_demo.hnep");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let profile = foreign_profile()?;
    profile.write_to(out)?;
    println!("wrote        {}", out.display());
    println!("profile_fp   {}", profile.header.fingerprint);

    let d = Dispatcher::open(out, MismatchPolicy::FallbackBaseline)?;
    println!("status       {}", d.status().replace('\n', " | "));
    println!("baseline_only {}", d.using_baseline());
    println!("selected     {}", d.workload("demo"));
    assert!(
        d.using_baseline(),
        "foreign profile must force baseline on this host"
    );
    assert!(
        d.status().contains("PROFILE MISMATCH"),
        "status must report PROFILE MISMATCH"
    );

    println!();
    println!("CLI: cargo run -p silicera-cli -- verify out/foreign_demo.hnep");
    Ok(())
}

