# Getting started

## Prerequisites

- Rust 1.75+ (edition 2021)
- x86_64 host
- For full specialization: AMD Zen3, Zen4, or Zen5 CPU

## Build

From the repository root:

```bash
cargo build --release -p silicera-cli
```

## First commands

```bash
cargo run -p silicera-cli -- doctor
cargo run -p silicera-cli -- inspect
cargo run -p silicera-cli -- probe --packs profiles/amd
```

If the CPU is unsupported, Silicera prints a clear reason and exits with code 2.

## First profile

```bash
mkdir -p out
cargo run -p silicera-cli -- train -o out/profile.hnep --iterations 20
cargo run -p silicera-cli -- verify out/profile.hnep
cargo run -p silicera-cli -- doctor --profile out/profile.hnep
cargo run -p silicera-cli -- explain --profile out/profile.hnep --size 65536
```

All medians printed are from the measurement engine on **this** host. Train also measures L1/L2/L3/DRAM size classes into HNEP v2. `verify` / `doctor --profile` report staleness and retrain recommendations. Partial retrain: `train --only concurrency`.

## Silicon Split / harness / threads

```bash
cargo run -p silicera-cli -- silicon-split train --role A -o out/machine_a.hnep
cargo run -p silicera-cli -- compare out/machine_a.hnep --placeholder
cargo run -p silicera-cli -- harness --domain all --iterations 30
cargo run -p silicera-cli -- threads --iterations 15
```

Machine B remains a placeholder until a second Zen box trains with the same protocol — verdict stays UNKNOWN.

## Mock mode (CI / non-AMD)

```bash
cargo run -p silicera-cli -- inspect --mock zen5
cargo run -p silicera-cli -- inspect --mock unsupported
```

## Lab UI

```bash
cargo run -p silicera-cli -- lab
```

Press `R` to run a measurement pass; `Q` to quit. The UI never displays hard-coded speedups.

## Phase I demos

Runnable experiment paths (Silicon Split, Portable vs Native, Cold Machine, Wrong Machine):

```bash
cargo run -p silicera-lab --example silicon_split
cargo run -p silicera-lab --example portable_vs_native
cargo run -p silicera-lab --example cold_machine
cargo run -p silicera-lab --example wrong_machine
```

Recipes and CLI parallels: [examples/README.md](../../examples/README.md).
