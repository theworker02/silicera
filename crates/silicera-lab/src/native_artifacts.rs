//! Dual-artifact comparison: portable codegen vs `-C target-cpu=native`.
//!
//! Builds the standalone `benchmarks/arm_kernels` package twice (generic vs
//! native), runs each binary, and compares measured medians. This is the
//! dual-artifact track beyond in-process AVX2 stand-ins.
//!
//! Requires a working `cargo` on PATH. Never invents timings.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use silicera::hardware::HardwareInfo;
use silicera::Result;

/// One compiled artifact run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactArm {
    /// `portable` or `native`.
    pub arm: String,
    /// RUSTFLAGS used.
    pub rustflags: String,
    /// Path to binary (best-effort).
    pub binary: String,
    /// Kernel name requested.
    pub kernel: String,
    /// Iterations inside the binary.
    pub iterations: usize,
    /// Median nanoseconds reported by the binary (or wall-time fallback).
    pub median_ns: f64,
    /// Raw stdout (truncated).
    pub stdout_excerpt: String,
    /// Notes / methodology.
    pub notes: String,
}

/// Full dual-artifact report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeArtifactReport {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Kernel.
    pub kernel: String,
    /// Portable arm.
    pub portable: ArtifactArm,
    /// Native arm.
    pub native: ArtifactArm,
    /// Silicera selection under min_improvement.
    pub silicera_choice: String,
    /// True when native beat portable by threshold.
    pub native_wins: bool,
    /// True when portable is best.
    pub portable_wins: bool,
    /// Conclusion.
    pub conclusion: String,
}

/// Locate workspace root (directory containing `benchmarks/arm_kernels`).
pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    for _ in 0..8 {
        if cur.join("benchmarks").join("arm_kernels").join("Cargo.toml").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

/// Run portable vs native compiled artifact comparison.
pub fn run_native_artifact_compare(
    info: &HardwareInfo,
    kernel: &str,
    iterations: usize,
    min_improvement: f64,
    workspace: &Path,
) -> Result<NativeArtifactReport> {
    let kernels_dir = workspace.join("benchmarks").join("arm_kernels");
    if !kernels_dir.join("Cargo.toml").is_file() {
        return Err(silicera::SiliceraError::Parse(format!(
            "missing benchmarks/arm_kernels at {}",
            kernels_dir.display()
        )));
    }

    let target_root = workspace.join("target").join("silicera-arms");
    std::fs::create_dir_all(&target_root)?;

    let portable = build_and_run(
        &kernels_dir,
        &target_root.join("portable"),
        "portable",
        "-C target-cpu=x86-64-v2",
        kernel,
        iterations,
    )?;
    let native = build_and_run(
        &kernels_dir,
        &target_root.join("native"),
        "native",
        "-C target-cpu=native",
        kernel,
        iterations,
    )?;

    let native_wins =
        native.median_ns < portable.median_ns * (1.0 - min_improvement);
    let portable_wins = portable.median_ns <= native.median_ns;
    let silicera_choice = if native_wins {
        "silicera→native_artifact".into()
    } else {
        "silicera→portable_artifact".into()
    };
    let conclusion = if native_wins {
        format!(
            "Native artifact faster ({:.0} vs {:.0} ns, min_improvement={}). \
             Silicera would prefer the native-compiled kernel for this host/kernel.",
            native.median_ns, portable.median_ns, min_improvement
        )
    } else if portable_wins {
        format!(
            "Portable artifact ≤ native ({:.0} vs {:.0} ns). \
             No specialization win from -C target-cpu=native under these knobs — report honestly.",
            portable.median_ns, native.median_ns
        )
    } else {
        format!(
            "Difference below threshold. portable={:.0} native={:.0} ns.",
            portable.median_ns, native.median_ns
        )
    };

    Ok(NativeArtifactReport {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        kernel: kernel.into(),
        portable,
        native,
        silicera_choice,
        native_wins,
        portable_wins,
        conclusion,
    })
}

/// Default kernels for a single-machine artifact suite.
pub fn default_artifact_kernels() -> &'static [&'static str] {
    &["dot_f32", "saxpy_f32", "checksum_u8", "reduce_i32"]
}

/// Suite of native-artifact comparisons on one host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeArtifactSuite {
    /// Host brand.
    pub host_brand: String,
    /// Fingerprint.
    pub fingerprint: Option<String>,
    /// Per-kernel reports.
    pub reports: Vec<NativeArtifactReport>,
    /// Kernels where native won.
    pub native_win_count: usize,
    /// Kernels where portable won / tied.
    pub portable_win_count: usize,
    /// Summary line.
    pub summary: String,
}

/// Run all default (or provided) kernels.
pub fn run_native_artifact_suite(
    info: &HardwareInfo,
    kernels: &[&str],
    iterations: usize,
    min_improvement: f64,
    workspace: &Path,
) -> Result<NativeArtifactSuite> {
    let mut reports = Vec::new();
    for k in kernels {
        reports.push(run_native_artifact_compare(
            info,
            k,
            iterations,
            min_improvement,
            workspace,
        )?);
    }
    let native_win_count = reports.iter().filter(|r| r.native_wins).count();
    let portable_win_count = reports.iter().filter(|r| r.portable_wins).count();
    let summary = format!(
        "artifact suite: native_wins={native_win_count} portable_wins={portable_win_count} kernels={}",
        reports.len()
    );
    Ok(NativeArtifactSuite {
        host_brand: info.brand.clone(),
        fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
        reports,
        native_win_count,
        portable_win_count,
        summary,
    })
}

/// Convert a suite into HNEP workload entries (names `artifact-<kernel>`).
pub fn suite_to_workload_entries(suite: &NativeArtifactSuite) -> Vec<silicera::WorkloadEntry> {
    use silicera::{Confidence, WorkloadEntry};
    suite
        .reports
        .iter()
        .map(|r| {
            let (winner, wmed, bmed, conf) = if r.native_wins {
                (
                    "native_artifact",
                    Some(r.native.median_ns),
                    Some(r.portable.median_ns),
                    Confidence::High,
                )
            } else if r.portable_wins {
                (
                    "portable_artifact",
                    Some(r.portable.median_ns),
                    Some(r.native.median_ns),
                    Confidence::Medium,
                )
            } else {
                (
                    "portable_artifact",
                    Some(r.portable.median_ns),
                    Some(r.native.median_ns),
                    Confidence::Low,
                )
            };
            WorkloadEntry {
                name: format!("artifact-{}", r.kernel),
                winner: winner.into(),
                confidence: conf,
                rationale: r.conclusion.clone(),
                winner_median_ns: wmed,
                baseline_median_ns: bmed,
            }
        })
        .collect()
}

fn build_and_run(
    kernels_dir: &Path,
    target_dir: &Path,
    arm: &str,
    rustflags: &str,
    kernel: &str,
    iterations: usize,
) -> Result<ArtifactArm> {
    std::fs::create_dir_all(target_dir)?;
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(kernels_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", target_dir)
        .env("RUSTFLAGS", rustflags)
        .status()
        .map_err(|e| silicera::SiliceraError::Parse(format!("cargo build failed to start: {e}")))?;
    if !status.success() {
        return Err(silicera::SiliceraError::Parse(format!(
            "cargo build ({arm}) failed with {status}"
        )));
    }

    let bin = target_dir
        .join("release")
        .join(if cfg!(windows) {
            "silicera-arm-kernels.exe"
        } else {
            "silicera-arm-kernels"
        });
    if !bin.is_file() {
        return Err(silicera::SiliceraError::Parse(format!(
            "expected binary at {}",
            bin.display()
        )));
    }

    let t0 = Instant::now();
    let output = Command::new(&bin)
        .arg("--kernel")
        .arg(kernel)
        .arg("--iterations")
        .arg(iterations.to_string())
        .output()
        .map_err(|e| silicera::SiliceraError::Parse(format!("run {arm}: {e}")))?;
    let wall_ns = t0.elapsed().as_nanos() as f64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(silicera::SiliceraError::Parse(format!(
            "{arm} binary failed: {stderr}"
        )));
    }
    let median_ns = parse_median_ns(&stdout).unwrap_or(wall_ns);
    let excerpt: String = stdout.chars().take(400).collect();
    Ok(ArtifactArm {
        arm: arm.into(),
        rustflags: rustflags.into(),
        binary: bin.display().to_string(),
        kernel: kernel.into(),
        iterations,
        median_ns,
        stdout_excerpt: excerpt,
        notes: format!("built with RUSTFLAGS='{rustflags}'"),
    })
}

fn parse_median_ns(stdout: &str) -> Option<f64> {
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("median_ns=") {
            return rest.trim().parse().ok();
        }
    }
    None
}
