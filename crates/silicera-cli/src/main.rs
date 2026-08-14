//! silicera — command-line interface.
//!
//! Silicera — experimental hardware-native execution research for AMD Zen.
//! Not affiliated with Advanced Micro Devices, Inc.
//!
//! Organized into groups: `machine`, `hnep`, `measure`, `research`, `export`,
//! `meta`. A small set of flat aliases covers the highest-traffic verbs and CI.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use tracing_subscriber::EnvFilter;

mod catalog;
mod commands;
mod style;

use catalog::{print_commands, print_guide, print_recipe};
use commands::{
    cmd_about, cmd_align, cmd_arch_compare, cmd_archive, cmd_benchmark, cmd_calm_check, cmd_compare,
    cmd_counters, cmd_digest, cmd_dispatch, cmd_doctor, cmd_env, cmd_eval, cmd_experiment,
    cmd_explain, cmd_feedback, cmd_fingerprint, cmd_fleet_export, cmd_harness, cmd_health, cmd_init,
    cmd_inspect, cmd_lab, cmd_native_artifacts, cmd_packs, cmd_placement, cmd_probe, cmd_profile,
    cmd_remarks, cmd_remarks_summary, cmd_report, cmd_repro_export, cmd_retrain, cmd_runtime,
    cmd_schema, cmd_silicon_split, cmd_specialize, cmd_staleness, cmd_threads, cmd_toolchain,
    cmd_topology, cmd_tpm, cmd_train, cmd_tree, cmd_validate, cmd_verify,
};

/// Silicera — experimental hardware-native execution research for AMD Zen.
#[derive(Parser, Debug)]
#[command(
    name = "silicera",
    version,
    about = "Hardware-native program specialization for AMD Zen (research)",
    long_about = "Silicera treats the physical machine as an input to program specialization.\n\
                  Supports AMD Zen3/Zen4/Zen5. Not affiliated with AMD.\n\
                  Measurements are real; speedups are never fabricated.\n\n\
                  Discover:   silicera commands\n\
                  Guides:     silicera guide list\n\
                  Recipes:    silicera recipe list\n\
                  Groups:     machine | hnep | measure | research | export | meta",
    after_help = "Sponsor / thanks: https://thanks.dev/u/gh/theworker02\n\
                  Dual-licensed MIT OR Apache-2.0. Homepage: https://silicera.dev",
    propagate_version = true
)]
struct Cli {
    /// Increase logging verbosity (-v, -vv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Machine discovery, knowledge packs, and host health.
    #[command(subcommand)]
    Machine(MachineCmd),
    /// HNEP train / verify / dispatch / staleness.
    #[command(subcommand)]
    Hnep(HnepCmd),
    /// Benchmarks, harnesses, placement, artifacts.
    #[command(subcommand)]
    Measure(MeasureCmd),
    /// Single-Zen eval, lab UI, experiments, Silicon Split.
    #[command(subcommand)]
    Research(ResearchCmd),
    /// SCF, remarks, fleet, repro, results archive.
    #[command(subcommand)]
    Export(ExportCmd),
    /// About, guides, recipes, schema, completions.
    #[command(subcommand)]
    Meta(MetaCmd),

    // Flat aliases (keep this list small — large flat+nested trees overflow Windows stacks in clap).
    /// Alias → machine inspect
    Inspect {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mock: Option<String>,
    },
    /// Alias → hnep train
    Train {
        #[arg(short, long, default_value = "out/profile.hnep")]
        output: String,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long, default_value = "train")]
        label: String,
        #[arg(long)]
        only: Option<String>,
    },
    /// Alias → hnep verify
    Verify {
        profile: String,
        #[arg(long)]
        strict_machine: bool,
        #[arg(long)]
        spot_check: bool,
    },
    /// Alias → machine doctor
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Alias → research eval
    Eval {
        #[arg(short, long, default_value = "out/single-machine-eval.json")]
        output: String,
        #[arg(long, default_value_t = 15)]
        harness_iters: usize,
        #[arg(long, default_value_t = 25)]
        artifact_iters: usize,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        merge_profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Alias → research lab
    Lab,
    /// Alias → research arch-compare
    ArchCompare {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mock: bool,
    },
    /// Alias → measure harness
    Harness {
        #[arg(long, default_value = "all")]
        domain: String,
        #[arg(long, default_value_t = 30)]
        iterations: usize,
        #[arg(long, default_value_t = 5)]
        warmup: usize,
        #[arg(long)]
        json: bool,
    },
    /// Alias → measure calm-check
    CalmCheck {
        #[arg(long, default_value_t = 24)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
    /// Alias → export feedback
    Feedback {
        profile: String,
        #[arg(short, long, default_value = "out/feedback.scf.json")]
        output: String,
        #[arg(long)]
        json: bool,
    },
    /// Alias → meta about
    About {
        #[arg(long)]
        json: bool,
    },
    /// Alias → meta commands
    #[command(name = "commands", visible_alias = "help-commands")]
    Catalog {
        #[arg(long)]
        json: bool,
    },
    /// Alias → meta guide
    Guide {
        topic: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Alias → meta recipe
    Recipe {
        #[arg(default_value = "list")]
        action: String,
        id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Alias → meta report
    Report {
        #[arg(short, long, default_value = "out/host-report.md")]
        output: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Alias → meta toolchain
    Toolchain {
        #[arg(long)]
        json: bool,
    },
    /// Alias → hnep health
    Health {
        profile: String,
        #[arg(long)]
        json: bool,
    },
    /// Alias → export remarks-summary
    RemarksSummary {
        yaml: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum MachineCmd {
    Inspect {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mock: Option<String>,
    },
    Fingerprint {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mock: Option<String>,
    },
    Topology {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mock: Option<String>,
    },
    Env {
        #[arg(long)]
        json: bool,
    },
    Probe {
        #[arg(long, default_value = "profiles/amd")]
        packs: String,
        #[arg(long)]
        json: bool,
    },
    Packs {
        #[arg(default_value = "list")]
        action: String,
        #[arg(long)]
        dir: Option<String>,
        name: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    Counters {
        #[arg(long)]
        json: bool,
    },
    Tpm {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum HnepCmd {
    Train {
        #[arg(short, long, default_value = "out/profile.hnep")]
        output: String,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long, default_value = "train")]
        label: String,
        #[arg(long)]
        only: Option<String>,
    },
    #[command(visible_alias = "profile")]
    Show {
        path: String,
        #[arg(long)]
        json: bool,
    },
    Verify {
        profile: String,
        #[arg(long)]
        strict_machine: bool,
        #[arg(long)]
        spot_check: bool,
    },
    Validate {
        profile: String,
        #[arg(long)]
        json: bool,
    },
    Digest {
        profile: String,
        #[arg(long)]
        json: bool,
    },
    Explain {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        size: Option<u64>,
        #[arg(long)]
        workload: Option<String>,
    },
    Tree {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Dispatch {
        #[arg(long)]
        profile: String,
        #[arg(long)]
        workload: Option<String>,
        #[arg(long)]
        size: Option<u64>,
        #[arg(long)]
        strict_machine: bool,
        #[arg(long)]
        json: bool,
    },
    Specialize {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Staleness {
        profile: String,
        #[arg(long)]
        json: bool,
    },
    Retrain {
        profile: String,
        #[arg(long)]
        json: bool,
    },
    Health {
        profile: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum MeasureCmd {
    Benchmark {
        #[arg(long, default_value = "all")]
        domain: String,
        #[arg(long, default_value_t = 15)]
        iterations: usize,
    },
    Harness {
        #[arg(long, default_value = "all")]
        domain: String,
        #[arg(long, default_value_t = 30)]
        iterations: usize,
        #[arg(long, default_value_t = 5)]
        warmup: usize,
        #[arg(long)]
        json: bool,
    },
    NativeArtifacts {
        #[arg(long, default_value = "dot_f32")]
        kernel: String,
        #[arg(long, default_value_t = 40)]
        iterations: usize,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        suite: bool,
        #[arg(long)]
        merge_profile: Option<String>,
    },
    Threads {
        #[arg(long, default_value_t = 15)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
    Align {
        #[arg(long, default_value_t = 65536)]
        len: usize,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
    Placement {
        #[arg(long, default_value_t = 4)]
        threads: usize,
        #[arg(long, default_value_t = 15)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
    CalmCheck {
        #[arg(long, default_value_t = 24)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ResearchCmd {
    Eval {
        #[arg(short, long, default_value = "out/single-machine-eval.json")]
        output: String,
        #[arg(long, default_value_t = 15)]
        harness_iters: usize,
        #[arg(long, default_value_t = 25)]
        artifact_iters: usize,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        merge_profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Lab,
    Experiment {
        id: Option<String>,
        #[arg(long)]
        list: bool,
        #[arg(long, default_value_t = 15)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
    Compare {
        a: String,
        b: Option<String>,
        #[arg(long)]
        placeholder: bool,
        #[arg(long)]
        json: bool,
    },
    SiliconSplit {
        action: String,
        #[arg(long, default_value = "A")]
        role: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(short, long, default_value = "out/machine_a.hnep")]
        output: String,
        #[arg(long)]
        a: Option<String>,
        #[arg(long)]
        b: Option<String>,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long)]
        json: bool,
    },
    ArchCompare {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mock: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ExportCmd {
    Feedback {
        profile: String,
        #[arg(short, long, default_value = "out/feedback.scf.json")]
        output: String,
        #[arg(long)]
        json: bool,
    },
    Remarks {
        profile: String,
        #[arg(short, long, default_value = "out/remarks.yaml")]
        output: String,
        #[arg(long)]
        json: bool,
    },
    FleetExport {
        profile: String,
        #[arg(short, long, default_value = "out/fleet-share.json")]
        output: String,
        #[arg(long)]
        opt_in: bool,
    },
    ReproExport {
        #[arg(long, default_value = "manual")]
        experiment: String,
        #[arg(short, long, default_value = "out/repro.json")]
        output: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Archive {
        #[arg(long)]
        eval: Option<String>,
        #[arg(long, default_value = "benchmarks/results/archive.json")]
        index: String,
        #[arg(long)]
        list: bool,
        #[arg(long)]
        json: bool,
    },
    RemarksSummary {
        yaml: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum MetaCmd {
    About {
        #[arg(long)]
        json: bool,
    },
    #[command(name = "commands")]
    Catalog {
        #[arg(long)]
        json: bool,
    },
    Guide {
        topic: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Recipe {
        #[arg(default_value = "list")]
        action: String,
        id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Schema {
        #[arg(long)]
        json: bool,
    },
    Init {
        #[arg(default_value = "out")]
        dir: String,
    },
    Runtime {
        #[arg(long)]
        json: bool,
    },
    Completions { shell: String },
    Report {
        #[arg(short, long, default_value = "out/host-report.md")]
        output: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Toolchain {
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let filter = match cli.verbose {
        0 => EnvFilter::new("warn"),
        1 => EnvFilter::new("info"),
        _ => EnvFilter::new("debug"),
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();

    if let Err(e) = dispatch(cli.command) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn dispatch(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Machine(c) => dispatch_machine(c),
        Commands::Hnep(c) => dispatch_hnep(c),
        Commands::Measure(c) => dispatch_measure(c),
        Commands::Research(c) => dispatch_research(c),
        Commands::Export(c) => dispatch_export(c),
        Commands::Meta(c) => dispatch_meta(c),
        Commands::Inspect { json, mock } => cmd_inspect(json, mock.as_deref()),
        Commands::Train {
            output,
            iterations,
            label,
            only,
        } => cmd_train(&output, iterations, &label, only.as_deref()),
        Commands::Verify {
            profile,
            strict_machine,
            spot_check,
        } => cmd_verify(&profile, strict_machine, spot_check),
        Commands::Doctor { json, profile } => cmd_doctor(json, profile.as_deref()),
        Commands::Eval {
            output,
            harness_iters,
            artifact_iters,
            workspace,
            merge_profile,
            json,
        } => cmd_eval(
            &output,
            harness_iters,
            artifact_iters,
            workspace.as_deref(),
            merge_profile.as_deref(),
            json,
        ),
        Commands::Lab => cmd_lab(),
        Commands::ArchCompare { json, mock } => cmd_arch_compare(json, mock),
        Commands::Harness {
            domain,
            iterations,
            warmup,
            json,
        } => cmd_harness(&domain, iterations, warmup, json),
        Commands::CalmCheck { iterations, json } => cmd_calm_check(iterations, json),
        Commands::Feedback {
            profile,
            output,
            json,
        } => cmd_feedback(&profile, &output, json),
        Commands::About { json } => cmd_about(json),
        Commands::Catalog { json } => print_commands(json),
        Commands::Guide { topic, json } => print_guide(topic.as_deref(), json),
        Commands::Recipe { action, id, json } => print_recipe(&action, id.as_deref(), json),
        Commands::Report {
            output,
            profile,
            json,
        } => cmd_report(&output, profile.as_deref(), json),
        Commands::Toolchain { json } => cmd_toolchain(json),
        Commands::Health { profile, json } => cmd_health(&profile, json),
        Commands::RemarksSummary { yaml, json } => cmd_remarks_summary(&yaml, json),
    }
}

fn dispatch_machine(c: MachineCmd) -> anyhow::Result<()> {
    match c {
        MachineCmd::Inspect { json, mock } => cmd_inspect(json, mock.as_deref()),
        MachineCmd::Fingerprint { json, mock } => cmd_fingerprint(json, mock.as_deref()),
        MachineCmd::Topology { json, mock } => cmd_topology(json, mock.as_deref()),
        MachineCmd::Env { json } => cmd_env(json),
        MachineCmd::Probe { packs, json } => cmd_probe(&packs, json),
        MachineCmd::Packs {
            action,
            dir,
            name,
            json,
        } => cmd_packs(&action, dir.as_deref(), name.as_deref(), json),
        MachineCmd::Doctor { json, profile } => cmd_doctor(json, profile.as_deref()),
        MachineCmd::Counters { json } => cmd_counters(json),
        MachineCmd::Tpm { json } => cmd_tpm(json),
    }
}

fn dispatch_hnep(c: HnepCmd) -> anyhow::Result<()> {
    match c {
        HnepCmd::Train {
            output,
            iterations,
            label,
            only,
        } => cmd_train(&output, iterations, &label, only.as_deref()),
        HnepCmd::Show { path, json } => cmd_profile(&path, json),
        HnepCmd::Verify {
            profile,
            strict_machine,
            spot_check,
        } => cmd_verify(&profile, strict_machine, spot_check),
        HnepCmd::Validate { profile, json } => cmd_validate(&profile, json),
        HnepCmd::Digest { profile, json } => cmd_digest(&profile, json),
        HnepCmd::Explain {
            profile,
            size,
            workload,
        } => cmd_explain(profile.as_deref(), size, workload.as_deref()),
        HnepCmd::Tree { profile, json } => cmd_tree(profile.as_deref(), json),
        HnepCmd::Dispatch {
            profile,
            workload,
            size,
            strict_machine,
            json,
        } => cmd_dispatch(
            &profile,
            workload.as_deref(),
            size,
            strict_machine,
            json,
        ),
        HnepCmd::Specialize { profile, json } => cmd_specialize(profile.as_deref(), json),
        HnepCmd::Staleness { profile, json } => cmd_staleness(&profile, json),
        HnepCmd::Retrain { profile, json } => cmd_retrain(&profile, json),
        HnepCmd::Health { profile, json } => cmd_health(&profile, json),
    }
}

fn dispatch_measure(c: MeasureCmd) -> anyhow::Result<()> {
    match c {
        MeasureCmd::Benchmark { domain, iterations } => cmd_benchmark(&domain, iterations),
        MeasureCmd::Harness {
            domain,
            iterations,
            warmup,
            json,
        } => cmd_harness(&domain, iterations, warmup, json),
        MeasureCmd::NativeArtifacts {
            kernel,
            iterations,
            json,
            workspace,
            suite,
            merge_profile,
        } => cmd_native_artifacts(
            &kernel,
            iterations,
            json,
            workspace.as_deref(),
            suite,
            merge_profile.as_deref(),
        ),
        MeasureCmd::Threads { iterations, json } => cmd_threads(iterations, json),
        MeasureCmd::Align {
            len,
            iterations,
            json,
        } => cmd_align(len, iterations, json),
        MeasureCmd::Placement {
            threads,
            iterations,
            json,
        } => cmd_placement(threads, iterations, json),
        MeasureCmd::CalmCheck { iterations, json } => cmd_calm_check(iterations, json),
    }
}

fn dispatch_research(c: ResearchCmd) -> anyhow::Result<()> {
    match c {
        ResearchCmd::Eval {
            output,
            harness_iters,
            artifact_iters,
            workspace,
            merge_profile,
            json,
        } => cmd_eval(
            &output,
            harness_iters,
            artifact_iters,
            workspace.as_deref(),
            merge_profile.as_deref(),
            json,
        ),
        ResearchCmd::Lab => cmd_lab(),
        ResearchCmd::Experiment {
            id,
            list,
            iterations,
            json,
        } => cmd_experiment(id.as_deref(), list, iterations, json),
        ResearchCmd::Compare {
            a,
            b,
            placeholder,
            json,
        } => cmd_compare(&a, b.as_deref(), json, placeholder),
        ResearchCmd::SiliconSplit {
            action,
            role,
            profile,
            output,
            a,
            b,
            iterations,
            json,
        } => cmd_silicon_split(
            &action,
            &role,
            profile.as_deref(),
            &output,
            a.as_deref(),
            b.as_deref(),
            iterations,
            json,
        ),
        ResearchCmd::ArchCompare { json, mock } => cmd_arch_compare(json, mock),
    }
}

fn dispatch_export(c: ExportCmd) -> anyhow::Result<()> {
    match c {
        ExportCmd::Feedback {
            profile,
            output,
            json,
        } => cmd_feedback(&profile, &output, json),
        ExportCmd::Remarks {
            profile,
            output,
            json,
        } => cmd_remarks(&profile, &output, json),
        ExportCmd::FleetExport {
            profile,
            output,
            opt_in,
        } => cmd_fleet_export(&profile, &output, opt_in),
        ExportCmd::ReproExport {
            experiment,
            output,
            profile,
            json,
        } => cmd_repro_export(&experiment, &output, profile.as_deref(), json),
        ExportCmd::Archive {
            eval,
            index,
            list,
            json,
        } => cmd_archive(eval.as_deref(), &index, list, json),
        ExportCmd::RemarksSummary { yaml, json } => cmd_remarks_summary(&yaml, json),
    }
}

fn dispatch_meta(c: MetaCmd) -> anyhow::Result<()> {
    match c {
        MetaCmd::About { json } => cmd_about(json),
        MetaCmd::Catalog { json } => print_commands(json),
        MetaCmd::Guide { topic, json } => print_guide(topic.as_deref(), json),
        MetaCmd::Recipe { action, id, json } => print_recipe(&action, id.as_deref(), json),
        MetaCmd::Schema { json } => cmd_schema(json),
        MetaCmd::Init { dir } => cmd_init(&dir),
        MetaCmd::Runtime { json } => cmd_runtime(json),
        MetaCmd::Completions { shell } => {
            use std::str::FromStr;
            let sh = Shell::from_str(&shell).unwrap_or_else(|_| {
                eprintln!("unknown shell '{shell}' (bash|zsh|fish|powershell|elvish)");
                std::process::exit(2);
            });
            let mut cmd = Cli::command();
            generate(sh, &mut cmd, "silicera", &mut std::io::stdout());
            Ok(())
        }
        MetaCmd::Report {
            output,
            profile,
            json,
        } => cmd_report(&output, profile.as_deref(), json),
        MetaCmd::Toolchain { json } => cmd_toolchain(json),
    }
}
