//! Built-in command catalog, guides, and recipes for the expansive CLI.

use silicera::{
    AFFILIATION_DISCLAIMER, BRAND_LINE, FUNDING_URL, HOMEPAGE, LICENSE, PHASE, RELEASE_LINE, VERSION,
};

/// One catalog row for `silicera commands`.
pub struct CatalogEntry {
    pub group: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
}

/// Full CLI surface (grouped + flat aliases).
pub fn catalog() -> &'static [CatalogEntry] {
    &[
        CatalogEntry {
            group: "meta",
            name: "meta about | about",
            summary: "Brand, license, phase, homepage, funding",
        },
        CatalogEntry {
            group: "meta",
            name: "meta commands | commands",
            summary: "List every subcommand with short summaries",
        },
        CatalogEntry {
            group: "meta",
            name: "meta guide | guide",
            summary: "Print a topical operator guide",
        },
        CatalogEntry {
            group: "meta",
            name: "meta recipe | recipe",
            summary: "Curated multi-step workflows (list|show|print)",
        },
        CatalogEntry {
            group: "meta",
            name: "meta schema",
            summary: "HNEP / SCF / remarks format versions",
        },
        CatalogEntry {
            group: "meta",
            name: "meta init",
            summary: "Scaffold out/ directories and a starter README",
        },
        CatalogEntry {
            group: "meta",
            name: "meta completions",
            summary: "Generate shell completions",
        },
        CatalogEntry {
            group: "meta",
            name: "meta runtime",
            summary: "silicera-runtime identity block for embedders",
        },
        CatalogEntry {
            group: "meta",
            name: "meta report | report",
            summary: "Markdown host report (optional HNEP section)",
        },
        CatalogEntry {
            group: "meta",
            name: "meta toolchain | toolchain",
            summary: "Probe rustc/cargo/clang/PGO/BOLT presence",
        },
        CatalogEntry {
            group: "machine",
            name: "machine inspect | inspect",
            summary: "CPU, topology, fingerprint, support status",
        },
        CatalogEntry {
            group: "machine",
            name: "machine fingerprint",
            summary: "Print machine fingerprint (not authentication)",
        },
        CatalogEntry {
            group: "machine",
            name: "machine topology",
            summary: "Detailed topology graph summary",
        },
        CatalogEntry {
            group: "machine",
            name: "machine env",
            summary: "OS / arch / CPU environment snapshot",
        },
        CatalogEntry {
            group: "machine",
            name: "machine probe",
            summary: "Load knowledge packs and validate host",
        },
        CatalogEntry {
            group: "machine",
            name: "machine packs",
            summary: "List or show AMD knowledge packs",
        },
        CatalogEntry {
            group: "machine",
            name: "machine doctor | doctor",
            summary: "Health checks, counters, TPM stub, staleness",
        },
        CatalogEntry {
            group: "machine",
            name: "machine counters",
            summary: "Hardware counter availability probe",
        },
        CatalogEntry {
            group: "machine",
            name: "machine tpm",
            summary: "TPM binding stub status (optional experiment)",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep train | train",
            summary: "Tournaments → write HNEP profile",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep show",
            summary: "Show HNEP metadata and winners",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep verify | verify",
            summary: "Integrity, match, staleness, optional spot-check",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep validate",
            summary: "Integrity-only check (no host match)",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep digest",
            summary: "Show or re-verify integrity digest",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep explain",
            summary: "Decision-tree walk / workload rationale",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep tree",
            summary: "Dump decision tree structure",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep dispatch",
            summary: "Select variant for workload or size (runtime)",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep specialize",
            summary: "Size-specialization plan from topology",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep staleness",
            summary: "Environment drift vs trained profile",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep retrain",
            summary: "Partial retrain plan from drift signals",
        },
        CatalogEntry {
            group: "hnep",
            name: "hnep health | health",
            summary: "Profile health score vs live host environment",
        },
        CatalogEntry {
            group: "measure",
            name: "measure benchmark",
            summary: "Domain microbenchmarks (measured)",
        },
        CatalogEntry {
            group: "measure",
            name: "measure harness | harness",
            summary: "Portable vs host-ISA vs Silicera arms",
        },
        CatalogEntry {
            group: "measure",
            name: "measure native-artifacts",
            summary: "Portable vs -C target-cpu=native binaries",
        },
        CatalogEntry {
            group: "measure",
            name: "measure threads",
            summary: "Thread-count sweep",
        },
        CatalogEntry {
            group: "measure",
            name: "measure align",
            summary: "Alignment-sensitivity experiment",
        },
        CatalogEntry {
            group: "measure",
            name: "measure placement",
            summary: "Core-placement research",
        },
        CatalogEntry {
            group: "measure",
            name: "measure calm-check | calm-check",
            summary: "Soft noise / calm probe before eval",
        },
        CatalogEntry {
            group: "research",
            name: "research eval | eval",
            summary: "Full single-Zen evaluation pack",
        },
        CatalogEntry {
            group: "research",
            name: "research lab | lab",
            summary: "Interactive terminal measurement UI",
        },
        CatalogEntry {
            group: "research",
            name: "research experiment",
            summary: "Named measurement experiment demos",
        },
        CatalogEntry {
            group: "research",
            name: "research compare",
            summary: "Compare two HNEPs / placeholder",
        },
        CatalogEntry {
            group: "research",
            name: "research silicon-split",
            summary: "Multi-machine protocol (needs Machine B)",
        },
        CatalogEntry {
            group: "research",
            name: "research arch-compare | arch-compare",
            summary: "Structural Zen4 vs Zen5 compare",
        },
        CatalogEntry {
            group: "export",
            name: "export feedback | feedback",
            summary: "SCF JSON from HNEP",
        },
        CatalogEntry {
            group: "export",
            name: "export remarks",
            summary: "LLVM-style YAML remarks",
        },
        CatalogEntry {
            group: "export",
            name: "export remarks-summary | remarks-summary",
            summary: "Summarize an existing remarks YAML by pass",
        },
        CatalogEntry {
            group: "export",
            name: "export fleet-export",
            summary: "Opt-in anonymized fleet share",
        },
        CatalogEntry {
            group: "export",
            name: "export repro-export",
            summary: "Reproducibility sidecar pack",
        },
        CatalogEntry {
            group: "export",
            name: "export archive",
            summary: "Curated measured-results index",
        },
    ]
}

/// Print the command catalog.
pub fn print_commands(json: bool) -> anyhow::Result<()> {
    if json {
        let rows: Vec<_> = catalog()
            .iter()
            .map(|e| {
                serde_json::json!({
                    "group": e.group,
                    "name": e.name,
                    "summary": e.summary,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    println!("{}", crate::style::header("silicera commands"));
    println!("  {}  {}  Phase {}  v{}", BRAND_LINE, RELEASE_LINE, PHASE, VERSION);
    println!();
    let mut current = "";
    for e in catalog() {
        if e.group != current {
            current = e.group;
            println!("  [{current}]");
        }
        println!("    {:<40} {}", e.name, e.summary);
    }
    println!();
    println!("  Groups: machine · hnep · measure · research · export · meta");
    println!("  Flat aliases: inspect train verify doctor eval lab harness report …");
    println!("  funding  {FUNDING_URL}");
    Ok(())
}

/// Guide topics.
pub fn guide_topics() -> &'static [(&'static str, &'static str)] {
    &[
        ("quickstart", "First-hour path on a supported Zen host"),
        ("single-zen", "Single-machine evaluation loop (this release)"),
        ("hnep", "Hardware-Native Execution Profile basics"),
        ("silicon-split", "Why Machine B is required for YES/NO"),
        ("security", "Fingerprint is not authentication"),
        ("funding", "Sponsors / thanks.dev"),
        ("ethics", "No fabricated speedups; report losses"),
    ]
}

/// Print a guide topic.
pub fn print_guide(topic: Option<&str>, json: bool) -> anyhow::Result<()> {
    if topic.is_none() || topic == Some("list") {
        if json {
            let rows: Vec<_> = guide_topics()
                .iter()
                .map(|(id, summary)| serde_json::json!({"id": id, "summary": summary}))
                .collect();
            println!("{}", serde_json::to_string_pretty(&rows)?);
            return Ok(());
        }
        println!("{}", crate::style::header("silicera guide"));
        println!("  topics:");
        for (id, summary) in guide_topics() {
            println!("    {id:<16} {summary}");
        }
        println!();
        println!("  usage: silicera guide <topic>");
        return Ok(());
    }
    let topic = topic.unwrap();
    let body = match topic {
        "quickstart" => QUICKSTART,
        "single-zen" => SINGLE_ZEN,
        "hnep" => HNEP_GUIDE,
        "silicon-split" => SILICON_SPLIT_GUIDE,
        "security" => SECURITY_GUIDE,
        "funding" => FUNDING_GUIDE,
        "ethics" => ETHICS_GUIDE,
        other => anyhow::bail!("unknown guide '{other}' (silicera guide list)"),
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "topic": topic,
                "body": body,
                "version": VERSION,
                "phase": PHASE,
            })
        );
        return Ok(());
    }
    println!("{}", crate::style::header(&format!("silicera guide · {topic}")));
    println!("{body}");
    Ok(())
}

const QUICKSTART: &str = r#"
  1. silicera doctor
  2. silicera inspect
  3. silicera calm-check
  4. silicera train -o out/profile.hnep
  5. silicera verify out/profile.hnep --spot-check
  6. silicera eval -o out/single-machine-eval.json
  7. silicera export archive --eval out/single-machine-eval.json

  Explore the full surface:
    silicera commands
    silicera hnep --help
    silicera machine packs list

  Mock-friendly CI:
    silicera inspect --mock zen5
    silicera arch-compare --mock
"#;

const SINGLE_ZEN: &str = r#"
  This release focuses on one physical Zen host as the optimization input.

  Loop:
    calm-check → harness → native-artifacts --suite → align → placement → threads
    (or: silicera eval)

  Silicon Split YES/NO is deferred until a second Zen machine trains Machine B.
  Do not invent cross-machine verdicts from a single host.
"#;

const HNEP_GUIDE: &str = r#"
  HNEP = Hardware-Native Execution Profile (schema v2).

    silicera meta schema
    silicera hnep show out/profile.hnep
    silicera hnep validate out/profile.hnep
    silicera hnep digest out/profile.hnep
    silicera hnep tree --profile out/profile.hnep
    silicera hnep dispatch --profile out/profile.hnep --size 1048576
"#;

const SILICON_SPLIT_GUIDE: &str = r#"
  Protocol (research):
    silicera research silicon-split train --role A -o out/machine_a.hnep
    # On Machine B (second Zen box):
    silicera research silicon-split train --role B -o out/machine_b.hnep
    silicera research silicon-split report --a out/machine_a.split.json --b out/machine_b.split.json

  Without Machine B, report/compare against placeholder → verdict UNKNOWN.
"#;

const SECURITY_GUIDE: &str = r#"
  Machine fingerprints identify silicon class / topology hash for profile matching.
  They are NOT authentication, attestation, or DRM.

  See docs/security/fingerprint.md
"#;

const FUNDING_GUIDE: &str = concat!(
    "\n  Sponsor via thanks.dev (registered path):\n    ",
    "https://thanks.dev/u/gh/theworker02",
    "\n  Also listed in .github/FUNDING.yml (`thanks_dev: u/gh/theworker02`)\n"
);

const ETHICS_GUIDE: &str = r#"
  Measured numbers only. Report losses (NATIVE BEATS SILICERA, PORTABLE WINS).
  No fabricated % speedups. No fake AMD affiliation.
"#;

/// A multi-step recipe.
pub struct Recipe {
    pub id: &'static str,
    pub title: &'static str,
    pub steps: &'static [&'static str],
}

/// Built-in recipes.
pub fn recipes() -> &'static [Recipe] {
    &[
        Recipe {
            id: "first-train",
            title: "First train + verify on this host",
            steps: &[
                "silicera doctor",
                "silicera inspect",
                "silicera train -o out/profile.hnep --iterations 20",
                "silicera verify out/profile.hnep --spot-check",
                "silicera hnep explain --profile out/profile.hnep --size 1048576",
            ],
        },
        Recipe {
            id: "single-eval",
            title: "Single-Zen evaluation pack",
            steps: &[
                "silicera calm-check",
                "silicera eval -o out/single-machine-eval.json",
                "silicera export archive --eval out/single-machine-eval.json",
                "silicera export repro-export --experiment single-eval -o out/repro.json",
            ],
        },
        Recipe {
            id: "export-compiler",
            title: "Export SCF + LLVM-style remarks",
            steps: &[
                "silicera train -o out/profile.hnep",
                "silicera feedback out/profile.hnep -o out/feedback.scf.json",
                "silicera export remarks out/profile.hnep -o out/remarks.yaml",
            ],
        },
        Recipe {
            id: "artifact-merge",
            title: "Native artifact suite merged into HNEP",
            steps: &[
                "silicera train -o out/profile.hnep",
                "silicera measure native-artifacts --suite --merge-profile out/profile.hnep",
                "silicera hnep show out/profile.hnep",
            ],
        },
        Recipe {
            id: "ci-mock",
            title: "CI-friendly mock path (no real AMD required)",
            steps: &[
                "silicera about --json",
                "silicera inspect --mock zen5 --json",
                "silicera arch-compare --mock --json",
                "silicera meta schema --json",
            ],
        },
    ]
}

/// List / show recipes (print steps; does not execute).
pub fn print_recipe(action: &str, id: Option<&str>, json: bool) -> anyhow::Result<()> {
    match action {
        "list" => {
            if json {
                let rows: Vec<_> = recipes()
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "title": r.title,
                            "steps": r.steps,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                println!("{}", crate::style::header("silicera recipe list"));
                for r in recipes() {
                    println!("  · {:<18} {}", r.id, r.title);
                }
                println!();
                println!("  show:  silicera recipe show <id>");
                println!("  print: silicera recipe print <id>   # shell-friendly lines only");
            }
            Ok(())
        }
        "show" | "print" => {
            let id = id.ok_or_else(|| anyhow::anyhow!("recipe id required"))?;
            let r = recipes()
                .iter()
                .find(|r| r.id == id)
                .ok_or_else(|| anyhow::anyhow!("unknown recipe '{id}'"))?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "id": r.id,
                        "title": r.title,
                        "steps": r.steps,
                    })
                );
                return Ok(());
            }
            if action == "show" {
                println!("{}", crate::style::header(&format!("silicera recipe · {}", r.id)));
                println!("  {}", r.title);
                println!();
            }
            for s in r.steps {
                println!("{s}");
            }
            Ok(())
        }
        other => anyhow::bail!("unknown recipe action '{other}' (list|show|print)"),
    }
}

/// Banner for `--version`-style meta.
pub fn print_banner() {
    println!("{BRAND_LINE}");
    println!("  version {VERSION}  phase {PHASE}  · {RELEASE_LINE}  license {LICENSE}");
    println!("  {HOMEPAGE}");
    println!("  {AFFILIATION_DISCLAIMER}");
    println!("  funding  {FUNDING_URL}");
}
