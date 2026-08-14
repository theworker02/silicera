# HNEP — overview

File extension: `.hnep`  
Format id: `silicera-hnep`  
Schema version: **2** (this tree)

> **Normative contract:** [`docs/research/hnep-spec.md`](../research/hnep-spec.md)  
> This page is the conceptual overview; the spec is what a future tool should implement.

## Contents

- Header (format, version, silicera version, timestamp, fingerprint, label)
- Environment snapshot
- Workload entries (winner, confidence, rationale, optional medians)
- **Size-class entries** (L1 / L2 / L3 / DRAM measured winners + thresholds) — v2
- Optional size-dependent decision tree (compiled from size-classes when present)
- SHA-256 integrity digest over the canonical payload

## Knowledge packs vs HNEP

| | Knowledge pack | HNEP |
|--|----------------|------|
| Location | `profiles/amd/*.toml` | `*.hnep` |
| Nature | Architecture facts | Measured decisions |
| Portability | Shared across SKUs of a microarch | Bound to fingerprint class |

## Integrity

On load, Silicera recomputes the digest and rejects tampered files. Integrity is **not** the same as TPM attestation (optional, stubbed).

## Mismatch & staleness

- Fingerprint mismatch at runtime → `PROFILE MISMATCH` → baseline (unless strict-machine).
- Staleness checks compare OS/arch and age against a policy (`check_staleness`).

## Size-class dispatch

Train measures memscan/copy variants at L1/L2/L3/DRAM working sets, writes winners into `size_classes`, and compiles thresholds into `decision_tree`. Runtime evaluates the tree by working-set size.

See also: [`docs/concepts/specialization.md`](specialization.md), [`docs/research/silicon-split.md`](../research/silicon-split.md).
