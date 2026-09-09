<p align="center">
  <img src="../../assets/logo.svg" alt="Silicera" width="96" height="96" />
</p>

# silicera-lab

[![crates.io](https://img.shields.io/crates/v/silicera-lab.svg)](https://crates.io/crates/silicera-lab)
[![docs](https://img.shields.io/badge/docs-online-1f6f78.svg)](https://theworker02.github.io/silicera/api/silicera_lab/)

Measurement laboratory: microbenchmarks, harness arms, alignment/placement **studies**, native-artifact compare, Silicon Split protocol helpers, and a terminal UI for **live measured** sessions only.

```toml
[dependencies]
silicera-lab = "0.2"
```

```bash
cargo test -p silicera-lab
cargo run -p silicera-lab --example portable_vs_native
```

Public study APIs (`measure_alignment`, `measure_placement`, `spot_check_profile`) are re-exported at the crate root from `studies`.

Logo: [`../../assets/logo.svg`](../../assets/logo.svg)

> Not affiliated with Advanced Micro Devices, Inc. · Dual MIT OR Apache-2.0  
> Sponsor: [https://thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)
