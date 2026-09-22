# Buyer evaluation â€” Identity & environment

## Goal

In 15â€“45 minutes, verify the Product builds or runs as documented and that proprietary notices are present.

## Steps

1. Confirm root `LICENSE` is proprietary and `ACQUISITION.md` exists.
2. Skim `README.md` install/run claims.
3. Execute:

```
```bash
cargo install silicera-cli
silicera about
```
```toml
[dependencies]
silicera = "0.2"
silicera-runtime = "0.2"   # optional: dispatch only
silicera-lab = "0.2"       # optional: measurement / tournaments
```
```bash
cargo build --release
cargo test --workspace
```
```
generic code
     Ã¢â€â€š
     Ã¢â€“Â¼
machine fingerprint + knowledge packs
     Ã¢â€â€š
     Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
     Ã¢â€“Â¼              Ã¢â€“Â¼              Ã¢â€“Â¼
 variant A      variant B      baseline
     Ã¢â€â€š              Ã¢â€â€š              Ã¢â€â€š
     Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¼Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
                    Ã¢â€“Â¼
          measure Ã‚Â· gate Ã‚Â· select
                    Ã¢â€â€š
                    Ã¢â€“Â¼
                 .hnep  Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€“Âº  runtime dispatch
                              (mismatch Ã¢â€ â€™ baseline)
```
```bash
cargo build --release
cargo test --workspace
```
```bash
cargo run -p silicera-cli -- measure native-artifacts --suite
```
```bash
```

4. Run tests if present (`npm test`, `pytest`, `cargo test`, `go test ./...`, etc.).
5. Record README vs observed behavior gaps in workpapers.

## Pass criteria

- [ ] Clone succeeds
- [ ] Documented happy path works **or** failure is explained
- [ ] Minimal path needs no surprise secrets
- [ ] License notices intact

*Updated: 2026-09-22*
