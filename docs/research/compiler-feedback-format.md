# Compiler feedback format (SCF)

**Status:** Phase II experimental  
**Format id:** `silicera-compiler-feedback`  
**Schema version:** `1`

SCF is a **standalone JSON document** derived from a measured HNEP. A future
compiler, remark consumer, or multiversioning pass can read it **without**
linking the Silicera Rust stack.

## Non-goals

- Not an LLVM ABI
- Not a performance marketing channel
- Not authentication
- Not a substitute for local measurement on the target machine

## Produce

```bash
silicera feedback out/profile.hnep -o out/feedback.scf.json
```

## Schema (v1)

| Field | Meaning |
|-------|---------|
| `format` | Must be `silicera-compiler-feedback` |
| `version` | Schema version (`1`) |
| `silicera_version` | Producer library version |
| `fingerprint` | `SLC:AMD:…` measurement environment id |
| `microarchitecture` | Optional (`zen5`, …) |
| `workloads[]` | Name, winner, confidence, optional medians, rationale |
| `size_classes[]` | Class, thresholds, winner, confidence |
| `fallback_variant` | Decision-tree fallback when present |
| `caveats[]` | Required consumer warnings |

### Example

```json
{
  "format": "silicera-compiler-feedback",
  "version": 1,
  "silicera_version": "0.1.0",
  "fingerprint": "SLC:AMD:ZEN5:…",
  "microarchitecture": "zen5",
  "workloads": [
    {
      "name": "concurrency",
      "winner": "candidate",
      "confidence": "HIGH",
      "winner_median_ns": 166450.0,
      "baseline_median_ns": 533600.0,
      "rationale": "…"
    }
  ],
  "size_classes": [
    {
      "class": "L2",
      "threshold_bytes": 1048576,
      "working_set_bytes": 524288,
      "winner": "copy",
      "confidence": "HIGH"
    }
  ],
  "fallback_variant": "baseline",
  "caveats": [
    "SCF is experimental; not an LLVM ABI.",
    "Winners are machine-local measurements; re-verify on the target host."
  ]
}
```

## Consumer rules

1. Treat every winner as a **candidate** until re-verified on the consuming host.
2. Prefer raw medians over percentages.
3. Ignore unknown fields for forward compatibility within the same major version.
4. Reject documents whose `format` does not match.

## Relation to HNEP

| Artifact | Role |
|----------|------|
| HNEP | Full machine-bound specialization profile (digest, env, tree) |
| SCF | Slim compiler-facing export of measured strategy hints |

See also: `docs/research/hnep-spec.md`, `docs/research/llvm-integration.md`.
