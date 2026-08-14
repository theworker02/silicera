# Support matrix (Zen3/Zen4/Zen5)

| Class | Behavior |
|-------|----------|
| Zen5 desktop (e.g. Ryzen 9000) | Supported when packs validate |
| Zen4 desktop / server models in pack ranges | Supported |
| Zen3 models in pack ranges | Supported |
| Zen2 / older AMD | `UnsupportedMicroarch` — clear message, exit 2 |
| Intel / other | `Unsupported` — clear message, exit 2 |
| Mock zen4/zen5 | CI path via `--mock` |

Hardware tests marked `#[ignore]` require a real supported AMD host:

```bash
cargo test -p silicera -- --ignored
```
