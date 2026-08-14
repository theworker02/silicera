<p align="center">
  <img src="../assets/logo.svg" alt="Silicera" width="72" height="72" />
</p>

# Fuzzing Silicera

Optional cargo-fuzz targets for HNEP parsing. Not required for normal development.

## Prerequisites

- Nightly Rust (cargo-fuzz requirement)
- [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz): `cargo install cargo-fuzz`
- libFuzzer-compatible toolchain (Linux/macOS are the usual hosts; Windows support varies)

## Run

From the repository root:

```bash
cargo fuzz run hnep_parse --fuzz-dir fuzz
```

Or from `fuzz/`:

```bash
cd fuzz
cargo fuzz run hnep_parse
```

Target source: `fuzz/fuzz_targets/hnep_parse.rs` — feeds arbitrary bytes into HNEP parse / integrity paths.

## Notes

- Keep corpora and crashes out of commits unless intentionally minimized and reviewed.
- Fuzzing is complementary to `cargo test --workspace`; it does not replace measurement honesty on real hardware.
- CI inclusion is optional (see `ROADMAP.md`).
