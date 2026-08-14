# Alignment-aware specialization

This release asks a narrow question:

> Does alignment materially change which implementation wins on this host?

If **no**, Silicera must not add dispatch complexity.

## Run

```bash
silicera measure align --len 65536 --iterations 20
```

Offsets probed: `0, 1, 7, 15, 63`.

## Interpretation

| Outcome | Action |
|---------|--------|
| Winners / timings stable across offsets | Do **not** add alignment to the decision tree |
| Winners flip or >10% timing shift | Document and optionally encode alignment in dispatch |

All numbers come from the measurement engine. No fabricated sensitivity claims.
