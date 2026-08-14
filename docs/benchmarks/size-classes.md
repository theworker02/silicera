# Size-class HNEPs

## Idea

Working-set size relative to L1 / L2 / L3 / DRAM can change which variant wins. Silicera measures each class, stores winners in HNEP `size_classes`, and compiles thresholds into `decision_tree`. Extended float/intmix size targets are recorded as workloads for evidence; the decision tree is driven by memscan size-classes.

## Train path

```bash
cargo run -p silicera-cli -- train -o out/profile.hnep --iterations 20
cargo run -p silicera-cli -- explain --profile out/profile.hnep --size 4096
cargo run -p silicera-cli -- train -o out/profile.hnep --only concurrency
```

`train` prints a **winner-change story** (whether L1/L2/L3/DRAM winners differ) and a **multi-size dispatch overhead** summary (per-size delta median/mean/min/max).

## When winners do / don’t change

On the developer Ryzen 9 9950X (2026-08-12 wave): memscan size-class winners were **identical** (all baseline under gates) and harness memop Silicera selection was **identical** across L1–DRAM (`→portable`). That is a measured fact for those stand-in kernels — not a claim that size never matters on other hosts or with LLVM-tuned variants.

## Runtime

`silicera-runtime` selects via `Dispatcher::size(bytes)` using the profile tree when the fingerprint matches.

## Honesty

Winners are host-measured. Different machines may differ — that is Silicon Split’s question, not assumed here.
