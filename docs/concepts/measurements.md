# Measurements

The measurement engine collects timed samples after warmup and reports:

| Field | Meaning |
|-------|---------|
| median / mean | Central tendency (ranking uses **median**) |
| variance / stddev | Dispersion |
| p25 / p75 / p95 / p99 | Percentiles |
| outliers | Tukey fence count |
| stability | `STABLE` / `NOISY` / `UNSTABLE` |

Instability flags are first-class. Noisy wins produce lower confidence rather than marketing copy.

**Methodology note:** wall-clock timers are sensitive to frequency scaling, background load, and thermal state. Capture environment snapshots with profiles; treat single-pass results as provisional.
