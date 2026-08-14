# Portable vs `-march=native` vs Silicera

## Goal

Compare three arms under **identical** inputs, sample counts, warmups, and improvement thresholds:

| Arm | Phase I meaning |
|-----|-----------------|
| **portable** | Generic / baseline kernel |
| **native** | Stand-in for host-tuned code (alternate algorithm). Not yet LLVM `-march=native` codegen (Phase II). |
| **silicera** | Measured **selection** among candidates (may equal portable or native; may lose) |

## Knobs (must match)

- `warmup` (default 5)
- `iterations` (default 30)
- `min_improvement` (default 0.03)

## How to run

```bash
cargo run -p silicera-cli -- harness --domain all --warmup 5 --iterations 30
cargo run -p silicera-cli -- harness --domain float
cargo run -p silicera-cli -- threads --iterations 15
cargo run -p silicera-lab --example portable_vs_native
```

Domains: `all|integer|memory|float|branch|concurrency`. `all` / `memory` run memop across L1/L2/L3/DRAM.

## Reporting rules

- Print **wins and losses**. If native beats Silicera’s dispatched median, say so explicitly (`NATIVE BEATS SILICERA`).
- If portable wins all arms, say so — no specialization gain under these knobs.
- Do not paste fabricated percentages into the site; only measured host rows.

## Relation to compiler flags

True `-march=native` / `target-cpu` builds require a compiler feedback loop (see `docs/research/llvm-integration.md`). Phase I isolates the **selection** question with stand-in kernels so the harness stays reproducible inside the four-crate workspace.
