# Changelog

All notable changes to Silicera are documented here.

Format inspired by [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.1.1] — 2026-08-13

### Changed

- **Logo system refresh:** filled die substrate, bond-pad hints, L3→L1 rings, 2×2 core grid with teal selected tile, refined inbound/outbound paths (`assets/` + `site/assets/`)
- **Homepage** metadata points at GitHub Pages (`https://theworker02.github.io/silicera/`) instead of unresolved `silicera.dev`
- **Site docs page:** live links to docs.rs crates and GitHub narrative docs (fixes dead “map-only” docs UX)
- Added `[package.metadata.docs.rs]` so docs.rs builds all features after crates.io publish

### Fixed

- docs.rs / shields.io “docs not found” for 0.1.0 — republish 0.1.1 + host rustdoc on GitHub Pages (`/api/`) so docs are online while docs.rs queues

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
