# Topology

Silicera models:

`Package → ComputeDomain → Core → Thread` plus `SharedCache` (L3 per domain).

On Zen desktop parts, a Compute Domain corresponds to a **CCD**-like grouping for specialization purposes. Derivation uses CPUID cache leaves and OS logical CPU counts; when leaves are incomplete, packs supply expected sizes and validation emits warnings.

This model is intentionally simpler than full AMD infinity-fabric documentation — sufficient for size-dependent dispatch research, not a silicon tapeout reference.
