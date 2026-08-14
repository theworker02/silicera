<p align="center">
  <img src="../../assets/logo.svg" alt="Silicera" width="96" height="96" />
</p>

# silicera-runtime

[![crates.io](https://img.shields.io/crates/v/silicera-runtime.svg)](https://crates.io/crates/silicera-runtime)
[![docs](https://img.shields.io/badge/docs-online-1f6f78.svg)](https://theworker02.github.io/silicera/api/silicera_runtime/)

Lightweight **HNEP load and cheap dispatch** for Silicera-specialized binaries.

```toml
[dependencies]
silicera-runtime = "0.1"
```

No lab / measurement dependency. On fingerprint mismatch the dispatcher falls back to baseline (or fails closed under strict-machine policy).

```bash
cargo test -p silicera-runtime
```

Optional feature: `c-abi` for experimental C bindings (`silicera_init` / `silicera_profile_load` / `silicera_variant_select`).

Logo: [`../../assets/logo.svg`](../../assets/logo.svg)

> Not affiliated with Advanced Micro Devices, Inc. · Dual MIT OR Apache-2.0  
> Sponsor: [https://thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)
