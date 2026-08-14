# Single-Zen evaluation (no second machine)

When only one AMD Zen host is available, Silicera’s research surface is still large:

| Capability | Command |
|------------|---------|
| Soft noise probe | `silicera calm-check` |
| Domain harness | `silicera harness --domain all` |
| Compiled portable vs native | `silicera measure native-artifacts --suite` |
| Alignment sensitivity | `silicera measure align` |
| Placement / threads | `silicera measure placement` / `silicera measure threads` |
| Train + explain | `silicera train` / `silicera hnep explain` |
| Profile health | `silicera health out/profile.hnep` |
| **One-shot pack** | `silicera eval -o out/single-machine-eval.json` |

## One-shot

```bash
cargo run -p silicera-cli -- eval \
  -o out/single-machine-eval.json \
  --merge-profile out/profile.hnep
```

Writes:

- `out/single-machine-eval.json` — measured report
- `out/single-machine-eval.repro.json` — reproducibility pack
- optional HNEP merge of `artifact-*` workloads

## Explicitly deferred (needs another Zen box)

- Silicon Split YES/NO (Machine A vs Machine B strategy divergence)
- Cross-SKU architecture measurement (beyond mock structural compare)

Negative and inconclusive single-machine results are first-class evidence.
