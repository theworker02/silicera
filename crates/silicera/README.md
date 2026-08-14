<p align="center">
  <img src="../../assets/logo.svg" alt="Silicera" width="96" height="96" />
</p>

# silicera

[![crates.io](https://img.shields.io/crates/v/silicera.svg)](https://crates.io/crates/silicera)
[![docs](https://img.shields.io/badge/docs-online-1f6f78.svg)](https://theworker02.github.io/silicera/api/silicera/)

Core library for **hardware-native program specialization** on AMD Zen3 / Zen4 / Zen5.

```toml
[dependencies]
silicera = "0.1"
```

Discovery, topology, measurement, HNEP profiles, specialization trees, compiler feedback (SCF), remarks, profile lifecycle health, and knowledge packs live here.

```bash
cargo test -p silicera
```

| Module | Role |
|--------|------|
| `hardware` / `topology` / `fingerprint` | Host identity |
| `measure` / `tournament` / `variant` | Measured selection |
| `hnep` / `specialize` | Profiles + decision trees |
| `lifecycle` | Integrity + confidence + staleness health |
| `feedback` / `remarks` | SCF + LLVM-style YAML remarks |
| `brand` | Shared product constants (`RELEASE_LINE`, logo path, funding, …) |

Logo assets: [`../../assets/logo.svg`](../../assets/logo.svg) · [`logo-banner.svg`](../../assets/logo-banner.svg)

> Not affiliated with Advanced Micro Devices, Inc. · Dual MIT OR Apache-2.0  
> Sponsor: [https://thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)
