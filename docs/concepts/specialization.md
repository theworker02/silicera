# Specialization

Specialization in Silicera means: **select a variant** for a workload (or working-set size) using a profile measured on a compatible machine class.

It is not:

- Recompiling with `-march=native` alone
- Blindly applying another SKU’s timings
- An LLM proposing code transforms

Pipeline:

1. Discover hardware → fingerprint  
2. Run tournaments under correctness + regression gates  
3. Emit HNEP (winners, confidence, optional decision tree)  
4. Runtime loads HNEP → dispatch or baseline  

See also: [variants.md](variants.md), [measurements.md](measurements.md), [hnep.md](hnep.md).
