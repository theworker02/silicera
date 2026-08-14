# HNEP format specification (normative)

**Status:** Phase I stable contract for schema version 2  
**Format id:** `silicera-hnep`  
**Wire encoding:** UTF-8 JSON (pretty or compact)  
**File extension:** `.hnep`

This document is the standalone contract a future tool could consume **without** the Rust stack. Implementation types live in `crates/silicera/src/hnep.rs` and must stay aligned with this text.

## 1. Versioning and compatibility

| Constant | Value | Meaning |
|----------|-------|---------|
| `format` | `"silicera-hnep"` | Magic; reject other values |
| `version` | `2` | Current writers |
| Min readable | `1` | Legacy files (no `size_classes` in digest) |
| Max readable | `2` | This library |

### Semver rules for the HNEP schema

- **Major (`version` integer):** Breaking payload shape or digest coverage changes.
- **v1 → v2:** Additive `size_classes` array included in the integrity payload. Readers supporting only v1 must refuse v2. Writers in this tree always emit v2.
- **Unknown fields:** Not required to round-trip. Tools should ignore unknown JSON keys when *reading for display*, but **must not** recompute digests after dropping fields.
- **Experimental:** Fields not listed here are experimental and may change without a version bump only inside pre-1.0 Silicera crates — once listed here, they follow the table above.

### Stable vs experimental

| Surface | Stability |
|---------|-----------|
| `format`, `version`, `fingerprint`, `digest` | Stable |
| `workloads[]`, confidence enum, decision tree node tags | Stable |
| `size_classes[]` | Stable as of v2 |
| TPM attestation bindings | Experimental (stub) |
| Silicon Split sanitized export wrapper | Stable protocol id `silicera-silicon-split/1` (separate from HNEP version) |

## 2. Top-level document

```json
{
  "header": { "...": "..." },
  "environment": { "...": "..." },
  "workloads": [ ],
  "size_classes": [ ],
  "decision_tree": null,
  "digest": { "alg": "sha256", "hex": "..." }
}
```

### Integrity digest

1. Build the **canonical payload** object containing exactly: `header`, `environment`, `workloads`, `size_classes`, `decision_tree`.
2. Serialize with the same field set as Silicera’s serde JSON (field order as struct definition; no `digest` key).
3. `digest.alg` must be `"sha256"`.
4. `digest.hex` is lowercase hex of SHA-256 over the payload bytes.
5. On load: recompute and compare; mismatch → reject.

**v1 migration:** v1 digests covered payload **without** `size_classes`. Loaders may verify the legacy payload, then upgrade in memory to v2 with `size_classes: []` and recompute before rewrite.

## 3. Header

| Field | Type | Notes |
|-------|------|-------|
| `format` | string | Must be `silicera-hnep` |
| `version` | u32 | Schema version |
| `silicera_version` | string | Producer crate semver (informational) |
| `created_at` | string | RFC3339 |
| `fingerprint` | string | `SLC:AMD:…` machine class id |
| `label` | string | Human label |

## 4. Environment snapshot

Captured at train time. Used for staleness (OS/arch/age). Fields include at least: `os`, `arch`, `logical_cpus`, `captured_at` (RFC3339). Additional keys may appear; do not invent host facts.

## 5. Workload entries

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | Workload id |
| `winner` | string | Variant id |
| `confidence` | enum | `HIGH` \| `MEDIUM` \| `LOW` \| `INCONCLUSIVE` |
| `rationale` | string | Human explanation |
| `winner_median_ns` | number\|null | Informational |
| `baseline_median_ns` | number\|null | Informational |

Medians are **not** portable marketing claims.

## 6. Size-class entries (v2)

| Field | Type | Notes |
|-------|------|-------|
| `class` | string | `L1`, `L2`, `L3`, or `DRAM` |
| `threshold_bytes` | u64 | Class boundary used in the tree |
| `working_set_bytes` | u64 | Size measured |
| `winner` | string | Variant id |
| `confidence` | enum | Same as workloads |
| `rationale` | string | |
| `winner_median_ns` | number\|null | |
| `baseline_median_ns` | number\|null | |

Different classes **may** have different winners. Empty array means no measured size-class specialization.

## 7. Decision tree

Optional. When present, runtime evaluates `evaluate(size_bytes)` without re-measuring.

Node tagged enum (`type` field):

- `size_branch` — `threshold_bytes`, `label`, `less`, `greater_or_equal`
- `select` — `variant`

Tree also stores `l1_bytes`, `l2_bytes`, `l3_bytes`, `fallback_variant`.

**Mismatch semantics (runtime, not HNEP file):** if host fingerprint does not match header fingerprint, specialized winners must not apply; use baseline unless strict-machine errors.

**Staleness:** if OS/arch changed or age exceeds policy, profile may be rejected via `StaleProfile`.

## 8. Minimal example (v2)

```json
{
  "header": {
    "format": "silicera-hnep",
    "version": 2,
    "silicera_version": "0.1.0",
    "created_at": "2026-08-12T00:00:00+00:00",
    "fingerprint": "SLC:AMD:ZEN5:1A:44:00:0000000000000000:0000000000000000",
    "label": "example-minimal"
  },
  "environment": {
    "os": "windows",
    "arch": "x86_64",
    "logical_cpus": 32,
    "captured_at": "2026-08-12T00:00:00+00:00"
  },
  "workloads": [
    {
      "name": "integer",
      "winner": "baseline",
      "confidence": "INCONCLUSIVE",
      "rationale": "example only — recompute digest before use",
      "winner_median_ns": null,
      "baseline_median_ns": null
    }
  ],
  "size_classes": [
    {
      "class": "L1",
      "threshold_bytes": 32768,
      "working_set_bytes": 16384,
      "winner": "baseline",
      "confidence": "INCONCLUSIVE",
      "rationale": "example placeholder class",
      "winner_median_ns": null,
      "baseline_median_ns": null
    }
  ],
  "decision_tree": null,
  "digest": {
    "alg": "sha256",
    "hex": "REPLACE_AFTER_SERIALIZING_PAYLOAD"
  }
}
```

> The `digest.hex` above is intentionally invalid until computed over the payload. Use `HnepProfile::from_tournaments` / `recompute_digest` rather than hand-editing digests.

## 9. Machine fingerprint binding

- HNEP binds to a **fingerprint class**, not a chassis serial.
- Exact match → specialized dispatch allowed.
- Same silicon class but different topology hash → mismatch → baseline (default policy).
- Different class → mismatch → baseline / strict error.

See `docs/security/fingerprint.md`.

## 10. Change log (format)

| Version | Date | Change |
|---------|------|--------|
| 1 | 2026-08-12 | Initial JSON HNEP + workloads + optional tree + digest |
| 2 | 2026-08-12 | `size_classes` in payload; digest coverage includes size classes |
