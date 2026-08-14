<p align="center">
  <img src="../assets/logo.svg" alt="Silicera" width="72" height="72" />
</p>

# Benchmarks

Measurement methodology and standalone kernels used by Silicera lab/CLI.

| Path | Role |
|------|------|
| [arm_kernels/](arm_kernels/) | Dual-artifact portable vs `-C target-cpu=native` binaries |
| [../docs/benchmarks/portable-vs-native.md](../docs/benchmarks/portable-vs-native.md) | In-process harness |
| [../docs/benchmarks/native-artifacts.md](../docs/benchmarks/native-artifacts.md) | Compiled artifact track |
| [../docs/benchmarks/pgo-bolt.md](../docs/benchmarks/pgo-bolt.md) | PGO/BOLT comparison protocol |
| [../docs/benchmarks/size-classes.md](../docs/benchmarks/size-classes.md) | L1–DRAM specialization |
| [../docs/benchmarks/alignment.md](../docs/benchmarks/alignment.md) | Alignment sensitivity |
| [../docs/benchmarks/placement.md](../docs/benchmarks/placement.md) | Core placement |

## Rules

- Publish only measured numbers with host fingerprint / Silicera version.
- Report losses (portable or native beating Silicera) clearly.
- `arm_kernels` is **not** a workspace member (keeps primary crate count at four).
- Probe optional toolchains with `silicera toolchain` before claiming PGO/BOLT tables.
