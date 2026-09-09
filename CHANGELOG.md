# Changelog

All notable changes to Silicera are documented here.

Format inspired by [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.2.0] — 2026-09-09

### Added

- Deterministic HNEP revision diff API and `hnep diff` command with JSON output
  and `--fail-on-change`; identity-based matching includes timing, confidence,
  environment and decision-tree changes without performance conclusions.
- Allocation-free guarded workload/size dispatch with application variant registries,
  workload confidence floors and serializable selection/fallback reasons.
- Hardware prototype engineering package: three staged designs, manufacturer
  quotation/deliverable brief, acceptance tests and proposed USB instrumentation contract.
  Hardware and instrumentation remain design proposals, not implemented devices.

### Fixed

- Direct `LoadedProfile::from_parts` calls now verify integrity and schema, including
  legacy profile migration, before allowing dispatch.
- Strict-machine mode now rejects hosts without a supported fingerprint.

### Compatibility

- HNEP wire schema remains v2 with legacy v1 migration. Existing unguarded selection
  APIs retain behavior; guarded selection is opt-in. Callers must provide a baseline
  implementation and filter registry IDs for actual host ISA support.
- Four published workspace crates move together to 0.2.0; no dependencies added.

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
