# Acquisition Brief â€” Identity & environment

**Date:** 2026-09-22  
**Repository:** https://github.com/theworker02/silicera  
**Default branch:** `main`  
**Primary language:** Rust  
**Status:** Diligence briefing only. **No acquisition has occurred** by virtue of this file.  
**License:** Proprietary â€” sale, written commercial license, or completed asset transfer required (see root `LICENSE`).  
**Valuation:** Not stated.  
**Contact:** GitHub [@theworker02](https://github.com/theworker02) Â· [thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)

> Cloning or forking this repository does **not** grant production, redistribution, SaaS, OEM, or commercial rights.

---

## 1. Executive thesis

<img src="assets/logo-banner.svg" alt="Silicera Ã¢â‚¬â€ hardware-native specialization" width="520" /> <strong>Independent systems research software</strong> for hardware-native program specialization on <strong>AMD Zen</strong>. <a href="https://crates.io/crates/silicera"><img src="https://img.shields.io/crates/v/silicera.svg" alt="crates.io silicera" /></a>

**Why a buyer cares:** Identity & environment packages transferable product IP â€” source, docs, in-repo brand assets, and a diligence room under `docs/acquisition/` â€” under a clear proprietary posture so diligence can proceed without mistaking the repo for open source.

---

## 2. Product snapshot

| Item | Detail |
|------|--------|
| Product | Identity & environment |
| Repo | `theworker02/silicera` |
| Language | Rust |
| Open source? | **No** â€” proprietary |
| Rightsholder | theworker02 |
| Diligence pack | `docs/acquisition/` |

### Capability highlights (from current materials)

- Review retraining with `silicera hnep diff before.hnep after.hnep --json`.
- Embedders can use `LoadedProfile::guarded_workload` and `guarded_size` to
- Direct runtime loads now verify profiles; strict-machine rejects unsupported hosts.
- [Hardware prototype brief](docs/hardware/README.md): staged AMD appliance design,
- Publish medians with host brand, fingerprint, OS, and iteration counts.
- Report losses clearly (`PORTABLE WINS`, `NATIVE BEATS SILICERA`).
- Never invent Machine B numbers or marketing percentages.
- Probe optional PGO/BOLT toolchains with `silicera toolchain` before claiming those tables.
- thanks.dev: [https://thanks.dev/u/gh/theworker02](https://thanks.dev/u/gh/theworker02)
- GitHub: [https://github.com/theworker02/silicera](https://github.com/theworker02/silicera)
- GitHub Sponsors: [@theworker02](https://github.com/sponsors/theworker02)
- Funding file: [`.github/FUNDING.yml`](.github/FUNDING.yml)

---

## 3. Problem / opportunity

Teams evaluating Identity & environment typically need either (a) a commercial right to run or embed it, or (b) outright ownership of the Product IP for strategic build-out. Public GitHub visibility without a proprietary license creates false assumptions about free production use. This brief and the linked data room make the commercial path explicit.

---

## 4. What ships today

Honest maturity: treat repository contents, README claims, tests, and release tags as the source of truth. Do not assume production customers, ARR, filed patents, or SLAs unless separately evidenced in diligence.

Typical transferable surfaces:

- Source tree and build/test scripts present in-repo
- Documentation and design notes
- Acquisition / diligence markdown under `docs/acquisition/`
- Branding assets committed to the repository (if any)

---

## 5. Demo / evaluation path (buyer)

Minimal path (no secrets required unless README says otherwise):

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

Extended evaluation: `docs/acquisition/BUYER_EVALUATION.md`. Written NDA / evaluation grants may be required for private materials.

---

## 6. What a transaction typically includes

Subject to definitive schedules:

| Included (typical) | Excluded (typical) |
|--------------------|--------------------|
| Repo materials + asserted original IP | Seller personal accounts / unrelated repos |
| Docs + diligence room at closing | Third-party dependency source under separate licenses |
| In-repo brand marks as assigned | Secrets without rotation plan |
| Know-how captured in docs | Fabricated revenue, user, or adoption metrics |

---

## 7. Suggested deal structures

| Structure | When it fits |
|-----------|--------------|
| Non-exclusive commercial license | Deploy/run under seat or environment terms |
| Exclusive field-of-use license | Buyer wants exclusivity; seller may retain entity |
| Asset / IP assignment | Buyer wants ownership of Materials outright |
| OEM / redistribution | Separate agreement â€” not implied here |

Commercial terms (price, earnouts, escrow) are negotiated under NDA with counsel.

---

## 8. Buyer diligence checklist

- [ ] Confirm Rightsholder identity and authority to sell/license
- [ ] Inventory Materials (`docs/acquisition/ASSET_INVENTORY.md`)
- [ ] Review IP posture (`IP_PROVENANCE.md`) and dependencies (`DEPENDENCY_INVENTORY.md`)
- [ ] Run evaluation script (`BUYER_EVALUATION.md`)
- [ ] Review risks (`RISK_REGISTER.md`)
- [ ] Agree transfer scope (`TRANSFER_MANIFEST.md`) and handoff (`HANDOFF_CHECKLIST.md`)
- [ ] Supersede root `LICENSE` at closing via definitive agreement

---

## 9. Related documents

| Document | Purpose |
|----------|---------|
| `LICENSE` | Proprietary â€” no default grant |
| `docs/acquisition/README.md` | Data-room index |
| `docs/acquisition/EXECUTIVE_SUMMARY.md` | One-page thesis |
| `README.md` | Product overview |
| `SECURITY.md` | Vulnerability reporting |
| `COMMERCIAL.md` | Licensing contact path |
| `.github/FUNDING.yml` | Sponsors / thanks.dev |

---

## 10. Disclaimer

This package is informational and **does not** create a binding offer, grant of rights, or investment advice. Engage counsel for any transaction.

---

*Document version: 2.0.0 / 2026-09-22 Â· Classification: acquisition briefing*
