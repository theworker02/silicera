# Variants

A **variant** is a named implementation of a workload. Phase I uses the `Variant` trait in `silicera`:

- Stable `VariantId`
- `run() -> Output` for timing and correctness
- Correctness compared to a baseline reference output

Tournaments may reject variants that:

- Fail correctness
- Regress vs baseline beyond threshold
- Produce `UNSTABLE` measurements (optional abort)

The baseline variant is the safe fallback for cold machines and profile mismatch.
