<p align="center">
  <img src="../assets/logo.svg" alt="Silicera" width="72" height="72" />
</p>

# Examples

Measured demos for Silicera (Zen3 / Zen4 / Zen5). Numbers are host-local — never invent Machine B results.

| Demo | Intent | Run |
|------|--------|-----|
| **Silicon Split** | Multi-machine protocol: train A, Machine B placeholder, compare when B exists | `cargo run -p silicera-lab --example silicon_split` |
| **Portable vs Native vs Silicera** | Identical knobs; Silicera = measured selection; print losses | `cargo run -p silicera-lab --example portable_vs_native` |
| **Cold Machine** | No HNEP → baseline only | `cargo run -p silicera-lab --example cold_machine` |
| **Wrong Machine** | Foreign fingerprint → `PROFILE MISMATCH` → baseline | `cargo run -p silicera-lab --example wrong_machine` |

## CLI recipes

```bash
cargo run -p silicera-cli -- doctor
cargo run -p silicera-cli -- inspect

# Silicon Split (Machine A + B placeholder → verdict UNKNOWN)
cargo run -p silicera-cli -- research silicon-split train --role A -o out/machine_a.hnep
cargo run -p silicera-cli -- research silicon-split report --a out/machine_a.hnep
cargo run -p silicera-cli -- research compare out/machine_a.hnep --placeholder

# Portable vs native vs Silicera
cargo run -p silicera-cli -- harness --domain all --iterations 30

# Size-class HNEP
mkdir -p out
cargo run -p silicera-cli -- train -o out/profile.hnep
cargo run -p silicera-cli -- hnep explain --profile out/profile.hnep --size 1048576
cargo run -p silicera-cli -- verify out/profile.hnep
cargo run -p silicera-cli -- health out/profile.hnep
```

On a second Zen host (Machine B):

```bash
cargo run -p silicera-cli -- research silicon-split train --role B -o out/machine_b.hnep
# copy machine_b.hnep back to Machine A, then:
cargo run -p silicera-cli -- research compare out/machine_a.hnep out/machine_b.hnep
```

Mock hosts (CI / non-AMD):

```bash
cargo run -p silicera-cli -- inspect --mock zen5
```

## Measurement culture

- Report medians with host brand, fingerprint, OS, and iteration counts.
- Silicon Split verdict is **UNKNOWN** until Machine B is measured — never invent B numbers.
- Mismatch must never silently apply another SKU’s winners.
