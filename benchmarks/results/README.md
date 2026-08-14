<p align="center">
  <img src="../../assets/logo.svg" alt="Silicera" width="72" height="72" />
</p>

# Measured results archive

Only **measured** campaigns. Never invent Machine B numbers or speedups.

| File | Role |
|------|------|
| `archive.json` | Index (`silicera-results-archive/1`) |
| `*.json` | Full eval / harness artifacts referenced by the index |

```bash
cargo run -p silicera-cli -- export archive --eval out/single-machine-eval.json
cargo run -p silicera-cli -- export archive --list
```
