<p align="center">
  <img src="assets/logo-banner.svg" alt="Silicera â€” hardware-native specialization" width="520" />
</p>

<p align="center">
  <strong>Independent systems research software</strong> for hardware-native program specialization on <strong>AMD Zen</strong>.
</p>

<p align="center">
  <a href="https://crates.io/crates/silicera"><img src="https://img.shields.io/crates/v/silicera.svg" alt="crates.io silicera" /></a>
  <a href="https://crates.io/crates/silicera-cli"><img src="https://img.shields.io/crates/v/silicera-cli.svg" alt="crates.io silicera-cli" /></a>
  <a href="https://theworker02.github.io/silicera/api/silicera/"><img src="https://img.shields.io/badge/docs-online-1f6f78.svg" alt="API docs" /></a>
  <a href="license-Proprietary%20(source--available)"><img src="https://img.shields.io/badge/license-Proprietary%20(source--available)%20OR%20Apache--2.0-blue.svg" alt="license" /></a>
</p>

<p align="center">
  Single-Zen Research Release Â· Zen3 / Zen4 / Zen5 Â· Dual MIT OR source-available proprietary<br/>
  <a href="https://github.com/theworker02/silicera">GitHub</a> Â·
  <a href="https://thanks.dev/u/gh/theworker02">Sponsor</a> Â·
  <a href="site/index.html">Research site</a>
</p>

---

## Install from crates.io

**CLI** (installs the `silicera` binary):

```bash
cargo install silicera-cli
silicera about
```

**Libraries** (add to your `Cargo.toml`):

```toml
[dependencies]
silicera = "0.2"
silicera-runtime = "0.2"   # optional: dispatch only
silicera-lab = "0.2"       # optional: measurement / tournaments
```

| Crate | crates.io | docs |
|-------|-----------|------|
| **silicera** | [crates.io/crates/silicera](https://crates.io/crates/silicera) | [API (GitHub Pages)](https://theworker02.github.io/silicera/api/silicera/) Â· [docs.rs](https://docs.rs/silicera) |
| **silicera-runtime** | [crates.io/crates/silicera-runtime](https://crates.io/crates/silicera-runtime) | [API](https://theworker02.github.io/silicera/api/silicera_runtime/) Â· [docs.rs](https://docs.rs/silicera-runtime) |
| **silicera-lab** | [crates.io/crates/silicera-lab](https://crates.io/crates/silicera-lab) | [API](https://theworker02.github.io/silicera/api/silicera_lab/) Â· [docs.rs](https://docs.rs/silicera-lab) |
| **silicera-cli** | [crates.io/crates/silicera-cli](https://crates.io/crates/silicera-cli) | [docs.rs](https://docs.rs/silicera-cli) |

Or build from this repository:

```bash
cargo build --release
cargo test --workspace
```

## What Silicera is

Silicera treats the **physical machine** â€” topology, cache geometry, and measured behavior â€” as an input to program specialization.

1. A **portable artifact** carries multiple implementation variants (baseline + candidates).
2. On a supported host, Silicera **discovers** CPUID / topology and validates it against AMD Zen **knowledge packs**.
3. Lab **tournaments** measure candidates with correctness gates and stability checks.
4. A Hardware-Native Execution Profile (**HNEP**) records which variants won for that **fingerprint class**, plus an optional size-class decision tree.
5. **`silicera-runtime`** loads the HNEP and dispatches cheaply. On fingerprint mismatch it falls back to **baseline** (or errors under `--strict-machine`).

```
generic code
     â”‚
     â–¼
machine fingerprint + knowledge packs
     â”‚
     â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
     â–¼              â–¼              â–¼
 variant A      variant B      baseline
     â”‚              â”‚              â”‚
     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                    â–¼
          measure Â· gate Â· select
                    â”‚
                    â–¼
                 .hnep  â”€â”€â–º  runtime dispatch
                              (mismatch â†’ baseline)
```

> **Not affiliated with, endorsed by, or certified by** Advanced Micro Devices, Inc. AMD, Ryzen, and Zen are trademarks of Advanced Micro Devices, Inc.

## Why this exists

`-march=native`, PGO, multiversioning, BOLT, and classical autotuners each capture part of the hardware story. Silicera asks a narrower question:

> If **machine identity and measured cache/topology behavior** are first-class inputs, can portable binaries still dispatch **machine-appropriate** variants â€” with correctness gates, statistical skepticism, and safe fallbacks?

| Approach | Optimizes for | Silicera difference |
|----------|---------------|---------------------|
| `-march=native` | ISA / microarch at compile time | Specialization from **runtime-measured** machine + profile |
| LLVM PGO / BOLT | Profiled control flow / layout | HNEP binds winners to a **fingerprint class** |
| CPU multiversioning | Feature leaves (AVXâ€¦) | Decision trees over **working-set vs cache geometry** |
| ATLAS / FFTW | Search at install/build | Portable artifact + **mismatch â†’ baseline** |

See [docs/research/silicera-paper.md](docs/research/silicera-paper.md) and [docs/INDEX.md](docs/INDEX.md).

## Profile revision and prototype update (0.2.0)

- Review retraining with `silicera hnep diff before.hnep after.hnep --json`.
  Add `--fail-on-change` for a CI gate. Entries match by name; timestamps and
  digests are excluded. Timing changes are observations, not speedup claims.
- Embedders can use `LoadedProfile::guarded_workload` and `guarded_size` to
  reject unavailable variants and receive structured fallback reasons.
- Direct runtime loads now verify profiles; strict-machine rejects unsupported hosts.
- [Hardware prototype brief](docs/hardware/README.md): staged AMD appliance design,
  manufacturer deliverables, instrumentation proposal and acceptance gates.
  These are engineering plans; no hardware has been manufactured or qualified.

See [profile revision workflows](docs/concepts/profile-revisions.md).

## Status

**This release (Single-Zen Research Release)** ships:

| Area | Capabilities |
|------|----------------|
| Discovery | Zen3 / Zen4 / Zen5 CPUID, topology graph, knowledge-pack validation, machine fingerprint |
| Measurement | Warmup/iterations, median/stability, tournaments with correctness + regression gates |
| Profiles | HNEP schema v2, size-class trees, integrity digests, SCF feedback, LLVM-style remarks |
| Runtime | Cheap dispatch, mismatch â†’ baseline, optional C ABI, profile health / staleness / retrain |
| Lab / CLI | Harness, native-artifacts, calm-check, eval pack, align/placement/threads studies, report, toolchain probe |
| Research protocol | Silicon Split *protocol* + Machine B placeholder (verdict **UNKNOWN** without a second Zen host) |

**Deferred:** Silicon Split YES/NO and cross-SKU divergence tables need a second measured Zen machine. Single-machine work is not blocked â€” use `silicera eval`.

**Non-goals:** kernel drivers, BIOS/firmware modification, overclocking, LLM-in-the-loop optimization, fabricated speedups, automatic cloud upload of profiles.

## Workspace

Exactly **four** primary crates (plus excluded `benchmarks/arm_kernels` and optional `fuzz/`):

| Crate | Role | crates.io | Local README |
|-------|------|-----------|--------------|
| [`silicera`](https://crates.io/crates/silicera) | Discovery, topology, measurement, HNEP, specialization, lifecycle health | [v0.2.0](https://crates.io/crates/silicera) | [crates/silicera/README.md](crates/silicera/README.md) |
| [`silicera-runtime`](https://crates.io/crates/silicera-runtime) | Profile load + cheap dispatch (no lab dependency) | [v0.2.0](https://crates.io/crates/silicera-runtime) | [crates/silicera-runtime/README.md](crates/silicera-runtime/README.md) |
| [`silicera-lab`](https://crates.io/crates/silicera-lab) | Microbenchmarks, tournaments, studies, terminal lab, experiments | [v0.2.0](https://crates.io/crates/silicera-lab) | [crates/silicera-lab/README.md](crates/silicera-lab/README.md) |
| [`silicera-cli`](https://crates.io/crates/silicera-cli) | `silicera` binary â€” grouped CLI + flat aliases | [v0.2.0](https://crates.io/crates/silicera-cli) | [crates/silicera-cli/README.md](crates/silicera-cli/README.md) |

### Brand assets

Canonical logos live under [`assets/`](assets/):

| File | Use |
|------|-----|
| [`logo.svg`](assets/logo.svg) | Primary mark (die + cache rings + selected path) |
| [`logo-mark.svg`](assets/logo-mark.svg) | Compact mark |
| [`logo-16.svg`](assets/logo-16.svg) | Favicon / nav |
| [`logo-banner.svg`](assets/logo-banner.svg) | README / docs banner |
| [`social-card.svg`](assets/social-card.svg) | Open Graph / social |

The same mark is copied to [`site/assets/`](site/assets/) for GitHub Pages and linked from every package README.

## Build from source

```bash
cargo build --release
cargo test --workspace
```

Binary: `target/release/silicera` (or install with `cargo install silicera-cli`).

Optional native-artifact suite (separate crate, not a workspace member):

```bash
cargo run -p silicera-cli -- measure native-artifacts --suite
```

## Quick start

```bash
# Identity & environment
cargo run -p silicera-cli -- about
cargo run -p silicera-cli -- commands
cargo run -p silicera-cli -- doctor
cargo run -p silicera-cli -- inspect
cargo run -p silicera-cli -- toolchain
cargo run -p silicera-cli -- report -o out/host-report.md

# Train â†’ verify â†’ health
cargo run -p silicera-cli -- train -o out/profile.hnep
cargo run -p silicera-cli -- verify out/profile.hnep --spot-check
cargo run -p silicera-cli -- health out/profile.hnep
cargo run -p silicera-cli -- hnep explain --profile out/profile.hnep --size 1048576

# Compiler-facing exports
cargo run -p silicera-cli -- feedback out/profile.hnep -o out/feedback.scf.json
cargo run -p silicera-cli -- export remarks out/profile.hnep -o out/remarks.yaml
cargo run -p silicera-cli -- remarks-summary out/remarks.yaml

# Single-Zen evaluation pack
cargo run -p silicera-cli -- calm-check
cargo run -p silicera-cli -- eval -o out/single-machine-eval.json
cargo run -p silicera-cli -- export archive --eval out/single-machine-eval.json
```

### Discover the CLI

```bash
silicera commands          # full catalog
silicera guide quickstart  # operator guide
silicera recipe list       # printable multi-step workflows
silicera hnep --help       # profile / dispatch / health
silicera measure --help    # harness / artifacts / studies
```

| Group | Commands |
|-------|----------|
| `machine` | inspect, fingerprint, topology, env, probe, packs, doctor, counters, tpm |
| `hnep` | train, show, verify, health, validate, digest, explain, tree, dispatch, specialize, staleness, retrain |
| `measure` | benchmark, harness, native-artifacts, threads, align, placement, calm-check |
| `research` | eval, lab, experiment, compare, silicon-split, arch-compare |
| `export` | feedback, remarks, remarks-summary, fleet-export, repro-export, archive |
| `meta` | about, commands, guide, recipe, schema, init, runtime, report, toolchain, completions |

**Flat aliases** (CI / muscle memory): `inspect`, `train`, `verify`, `doctor`, `eval`, `lab`, `harness`, `calm-check`, `feedback`, `arch-compare`, `about`, `commands`, `guide`, `recipe`, `report`, `toolchain`, `health`, `remarks-summary`.

**Grouped-only** (no flat alias): `export remarks`, `export archive`, `research silicon-split`, `research compare`, `hnep explain`, `measure native-artifacts`, `measure align`, `measure placement`.

### CI / non-AMD hosts

Unsupported CPUs exit with a clear message (**exit code 2**), not a crash:

```bash
cargo run -p silicera-cli -- inspect --mock zen5
cargo run -p silicera-cli -- inspect --mock unsupported
cargo run -p silicera-cli -- arch-compare --mock
```

## Demo output (developer host)

Captured with `silicera doctor` / `inspect` on an **AMD Ryzen 9 9950X** (2026-08-12). Re-run on your machine; fingerprints are host-specific.

```
silicera doctor
  version            0.1.0
  phase              II
  host               AMD Ryzen 9 9950X 16-Core Processor
  support            supported AMD microarchitecture: ZEN5
  fingerprint        SLC:AMD:ZEN5:1A:44:00:567dcabfdc75b2c3:cbb6a67840c83bca
  env                windows / x86_64 / 32 cpus
  tpm                Stubbed
```

**Silicon Split:** Machine A can be trained; Machine B remains a placeholder until a second Zen host exists â†’ verdict **UNKNOWN**.

**Do not paste fabricated speedups.** Measured campaign notes live in [docs/research/silicera-paper.md](docs/research/silicera-paper.md) and [CHANGELOG.md](CHANGELOG.md).

## Core concepts

### HNEP vs knowledge packs

| Artifact | Path | Role |
|----------|------|------|
| Knowledge pack | `profiles/amd/*.toml` | Public architecture facts for **validation** (families, cache expectations) |
| HNEP | `*.hnep` | **Measured** winners + decision tree + integrity digest for a fingerprint |

Fingerprints identify a **machine class** for profile matching. They are **not authentication**. See [docs/security/fingerprint.md](docs/security/fingerprint.md).

### Measurement culture

- Publish medians with host brand, fingerprint, OS, and iteration counts.
- Report losses clearly (`PORTABLE WINS`, `NATIVE BEATS SILICERA`).
- Never invent Machine B numbers or marketing percentages.
- Probe optional PGO/BOLT toolchains with `silicera toolchain` before claiming those tables.

### Profile lifecycle

After training:

```bash
silicera verify out/profile.hnep --spot-check
silicera health out/profile.hnep          # 0â€“100 integrity + confidence + staleness
silicera hnep staleness out/profile.hnep
silicera hnep retrain out/profile.hnep    # partial --only plan when drift is soft
```

## Documentation map

| Doc | Audience |
|-----|----------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design |
| [ROADMAP.md](ROADMAP.md) | Current release Â· next (second Zen host) Â· hardening |
| [docs/INDEX.md](docs/INDEX.md) | Full doc hub |
| [docs/getting-started/index.md](docs/getting-started/index.md) | First measurements |
| [docs/benchmarks/single-machine.md](docs/benchmarks/single-machine.md) | Primary evidence path |
| [examples/](examples/) | Silicon Split, Portable vs Native, Cold / Wrong Machine |
| [site/](site/) | Static research site (GitHub Pages from `site/`) |
| [CHANGELOG.md](CHANGELOG.md) | Release history |

## Sponsor

Independent systems research software â€” dual-licensed MIT OR source-available proprietary.

- thanks.dev: [https://thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)
- GitHub: [https://github.com/theworker02/silicera](https://github.com/theworker02/silicera)
- GitHub Sponsors: [@theworker02](https://github.com/sponsors/theworker02)
- Funding file: [`.github/FUNDING.yml`](.github/FUNDING.yml)

```bash
cargo run -p silicera-cli -- about
```

## License

**Source-available proprietary** — evaluation under [LICENSE](./LICENSE); commercial / production use via [COMMERCIAL.md](./COMMERCIAL.md). See [LICENSE_TRANSITION_NOTICE.md](./LICENSE_TRANSITION_NOTICE.md) and [NOTICE](./NOTICE).


## Security

See [SECURITY.md](SECURITY.md). User-mode only. Fingerprints are not authentication. Report issues responsibly.
