# Silicera research notes (Phase I)

**Title working draft:** *Hardware-Native Execution Profiles: Treating the Machine as an Input to Program Specialization on AMD Zen*

**Status:** Research software notes — not a peer-reviewed publication.  
**Affiliation:** Independent. Not an AMD or LLVM Foundation document.

## Abstract (draft)

Profile-guided optimization, binary optimization, install-time autotuning, and ISA multiversioning each encode hardware assumptions at different binding times. This work explores a complementary binding: a portable program carries variants; a **Hardware-Native Execution Profile (HNEP)** records measured winners for a machine fingerprint class derived from topology and cache geometry; a lightweight runtime dispatches with mandatory fallback on mismatch. Phase I implements the measurement and profile machinery for AMD Zen3–Zen5 without kernel components.

## Related work and differentiation

| Line of work | Representative refs / practice | How Silicera differs |
|--------------|--------------------------------|----------------------|
| PGO | GCC/LLVM instrumentation & sampling PGO | HNEP selects among **explicit variants** bound to a **fingerprint**, not only edge/block weights |
| LLVM PGO | LLVM InstrProf / CSPGO | Same; Phase II may *consume* LLVM profiles as inputs, not replace them |
| BOLT | Binary layout optimization using samples | BOLT rewrites layout; Silicera selects variants + size trees without claiming BOLT’s domain |
| `-march=native` | Compile-time ISA/microarch | Binding at **profile load / run**, portable artifact |
| CPU multiversioning | `target_clones`, IFUNC | Feature-leaf dispatch vs **cache-geometry / measured** dispatch |
| ATLAS | Empirical search for BLAS kernels | Search produces HNEP for a machine class; mismatch fails safe |
| FFTW | Plan search / wisdom | Wisdom-like artifact with integrity digest + explicit confidence |
| Hardware counters | PMCs, `perf` | Phase I uses timers; counters optional / unavailable without privileges |
| Autotuning frameworks | OpenTuner et al. | No LLM; correctness/regression gates; AMD knowledge-pack validation |

Citations above are **landmarks** for readers; a formal paper should expand bibliographic entries. Phase I prioritizes a reproducible implementation over a literature survey.

## Evaluation (9950X host — measured 2026-08-12)

Host: **AMD Ryzen 9 9950X** · fingerprint `SLC:AMD:ZEN5:1A:44:00:567dcabfdc75b2c3:cbb6a67840c83bca` · Windows / x86_64 / 32 logical CPUs. Phase I “native” is a **stand-in kernel**, not LLVM `-march=native` codegen.

### Harness (`silicera harness --domain all --warmup 5 --iterations 30`)

| Workload | Portable ns | Native ns | Silicera | Outcome |
|----------|-------------|-----------|----------|---------|
| integer-mix | 950 | 11300 | →portable | Portable wins (no specialization gain) |
| float-sum | 3100 | 7100 | →portable | Portable wins |
| branch-mix | 3400 | 7500 | →portable | Portable wins |
| concurrency-atomics | 533600 | 166450 | →native | **Win:** selection after measurement |
| memop-L1 | 100 | 2300 | →portable | Portable wins (L1 medians noisy) |
| memop-L2 | 1600 | 49200 | →portable | Portable wins |
| memop-L3 | 105900 | 1580550 | →portable | Portable wins |
| memop-DRAM | 2147750 | 12749050 | →portable | Portable wins |

Suite: `portable_wins=7`, `native_beats_silicera=0`. Memop Silicera selection **identical across L1/L2/L3/DRAM** (`→portable`) on this run — winners do **not** change by size for this stand-in pool.

### Size-class train (`silicera train --iterations 20`)

Memscan L1/L2/L3/DRAM all kept **baseline** (INCONCLUSIVE/LOW — gates rejected noisy/non-improving candidates). Extended float/intmix size targets likewise inconclusive. **Concurrency** selected `candidate` at HIGH (+69% median vs contended atomics on that train). Decision-tree dispatch overhead: per-size delta median ≈ **100 ns** (batch stand-in can collapse to ~0 ns under release LTO — report per-size).

### Thread-count sweep (`silicera threads`, constant total work 80k iters)

| Threads | Baseline median ns | Candidate median ns |
|---------|--------------------|---------------------|
| 1 | 371700 | 77900 |
| 2 | 479000 | 117900 |
| 4 | 495700 | 184000 |
| 8 | 523700 | 335100 |
| 16 | 630100 | 624400 |
| 32 | 1198000 | 1125900 |

Best candidate and baseline medians at **threads=1** on this microbench — **more threads is not faster** here (spawn/sync dominate). No OS affinity changes.

### Silicon Split

Machine A refreshed on this host. Machine B = placeholder. Verdict **UNKNOWN**. Do not invent Machine B numbers.

### Honesty checklist

1. Environment snapshots accompany tables  
2. Medians + stability; no lone “X% faster” headlines for marketing  
3. Inconclusives published (most fixed-size ALU/memscan trains this wave)  
4. Wrong-machine / mismatch → baseline (runtime tests)  
5. Only measured numbers above  

## Limitations

- Topology from CPUID/OS may imperfectly reflect CCD boundaries  
- Timer noise under variable frequency; release LTO can zero trivial dispatch stand-ins  
- No LLVM codegen integration yet  
- TPM binding stubbed; HW counters UNAVAILABLE without privileges on Windows  
- Phase I native arm is not a real `-march=native` binary  

## HNE concept

See [hne-concept.md](hne-concept.md) and [compiler-feedback-format.md](compiler-feedback-format.md).
