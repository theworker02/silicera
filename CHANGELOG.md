# Changelog

All notable changes to Silicera are documented here.

Format inspired by [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased] — 2026-08-13

### Added

- **Published on crates.io (v0.1.0):** [`silicera`](https://crates.io/crates/silicera), [`silicera-runtime`](https://crates.io/crates/silicera-runtime), [`silicera-lab`](https://crates.io/crates/silicera-lab), [`silicera-cli`](https://crates.io/crates/silicera-cli) — install CLI with `cargo install silicera-cli`
- **Repository logo system:** silicon-die mark with cache rings + measured selection path (`assets/logo.svg`, `logo-mark.svg`, `logo-16.svg`, `logo-banner.svg`, `social-card.svg`); synced to `site/assets/` and all package READMEs
- **`silicera report`** — Markdown host report (`out/host-report.md` by default) with brand, doctor-style host facts, fingerprint note, optional `--profile` health, funding/disclaimer footer
- **`silicera toolchain`** — presence probe for rustc / cargo / clang / llvm-profdata / llvm-bolt (JSON flag; never invents PGO/BOLT results)
- **`silicera health <profile>`** — `ProfileHealth` score 0–100 (integrity + confidence + staleness) via `silicera::lifecycle`
- **`silicera remarks-summary <yaml>`** — counts remarks by pass name; `RemarksBundle::summary_lines()` / `summarize_yaml`
- Brand **`RELEASE_LINE`** (“Single-Zen Research Release”) and **`LOGO_RELATIVE`** for docs
- Per-crate READMEs with logo; expanded root README (concepts, CLI map, lifecycle, brand assets, crates.io links)

### Changed

- **Lab consolidation:** `alignment` / `placement` / `spot_check` merged into `silicera_lab::studies` (crate-root re-exports unchanged)
- **CLI consolidation:** `commands.rs` split into `commands/{machine,hnep,measure,research,export,meta,common}.rs`
- **Phase language merge:** user-facing “Phase I targets…” copy rewritten to Silicera / this release / Zen3–Zen5 wording; `PHASE` remains `"II"` internally
- **ROADMAP** restructured: current release checklist, next (second Zen host), hardening — not bolt-on “Phase III” marketing
- Crate `Cargo.toml` `readme` fields point at local crate READMEs

### Notes

- No fabricated speedups; not affiliated with AMD; thanks.dev remains `https://thanks.dev/u/gh/theworker02`
- Multi-machine Silicon Split YES/NO still deferred until a second Zen host exists

## [0.1.0] — 2026-08-12

### Added

- Workspace and AMD Zen3/4/5 knowledge packs
- HNEP, tournaments, runtime mismatch fallback, CLI/lab, docs/site

### Upgrades (absorbed into 0.1.0 line)

- HNEP schema v2, Silicon Split protocol, harness, size-class trees, staleness/partial retrain, C ABI, threads
- SCF feedback export; host-ISA AVX paths; align/placement/arch-compare/spot-check/fleet-share
- **Native artifacts:** `benchmarks/arm_kernels` + `silicera native-artifacts`
- **Repro packs:** `silicera repro-export`
- Host-ISA float dot + correctness gates; `specialize_target!` macro helper
- Docs: native-artifacts, pgo-bolt methodology
- GitHub Actions CI (mock hardware)
- **Single-Zen focus:** `silicera calm-check`, `native-artifacts --suite/--merge-profile`, `silicera eval` pack + repro sidecar; Machine B deferred in ROADMAP
- **Remarks:** `silicera remarks` → experimental LLVM-style YAML from HNEP
- **Archive:** `silicera archive` curated measured-results index
- **Completions:** `silicera completions <shell>`
- Professional hygiene: `docs/INDEX.md`, `CITATION.cff`, `rustfmt.toml`, `.editorconfig`, site refresh
- **Brand module:** `silicera::brand` (NAME/PHASE/TAGLINE/FUNDING_URL/THANKS_DEV) shared by CLI/lab/runtime
- **CLI `about`:** brand, license, phase, homepage, thanks.dev funding URL
- **Runtime `runtime_about()` / `RuntimeInfo`:** embedder identity + phase in mismatch diagnostics
- **Lab session export:** press `E` in lab UI → `out/lab-session.json`
- **Funding:** `.github/FUNDING.yml` (GitHub Sponsors `theworker02` + `thanks_dev: u/gh/theworker02`)
- **GitHub Pages:** `.github/workflows/pages.yml` deploys `site/`
- **Expansive CLI:** grouped surface (`machine` · `hnep` · `measure` · `research` · `export` · `meta`) plus flat aliases; `commands` / `guide` / `recipe` discovery

### Notes

- No fabricated speedups; not affiliated with AMD; phase marker `II`
- Multi-machine Silicon Split YES/NO deferred until a second Zen host exists
