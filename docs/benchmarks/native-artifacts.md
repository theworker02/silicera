# Native artifact comparison

Builds `benchmarks/arm_kernels` twice:

| Arm | RUSTFLAGS |
|-----|-----------|
| portable | `-C target-cpu=x86-64-v2` |
| native | `-C target-cpu=native` |

## Run

```bash
cargo run -p silicera-cli -- native-artifacts --kernel dot_f32 --iterations 40
```

Kernels: `dot_f32`, `saxpy_f32`, `checksum_u8`, `reduce_i32`.

Artifacts land under `target/silicera-arms/{portable,native}/`.

## Interpretation

- If native wins beyond `min_improvement`, Silicera records preference for the native artifact on this host.
- If portable wins, **report it** — that is evidence, not failure.
- This is distinct from in-process AVX2 (`HostIsaReduce` / `HostIsaDot`), which does not change LLVM codegen for the whole binary.
