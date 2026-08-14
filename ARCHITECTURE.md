# Architecture

Silicera architecture for AMD engineers, LLVM developers, and systems researchers.

**Brand line:** Silicera — experimental hardware-native execution research for AMD Zen.  
**Affiliation:** none with Advanced Micro Devices, Inc.  
**Evidence path:** single physical Zen host (multi-machine Silicon Split deferred).

## Design thesis

The physical machine is an **input** to specialization. A portable artifact carries multiple variants; a **Hardware-Native Execution Profile (HNEP)** records which variants won under measurement on a fingerprint class; the runtime selects cheaply and falls back to baseline on mismatch.

```
┌──────────────────────────────────────────────────────────────────────┐
│                         silicera-cli                                 │
│  inspect · train · eval · harness · native-artifacts · remarks · …   │
└───────────────┬───────────────────────────────┬──────────────────────┘
                │                               │
                ▼                               ▼
┌───────────────────────────┐     ┌───────────────────────────┐
│      silicera-lab         │     │    silicera-runtime       │
│  microbench · tournament  │     │  load HNEP · dispatch     │
│  profile gen · lab UI     │     │  mismatch → baseline      │
└─────────────┬─────────────┘     └─────────────▲─────────────┘
              │                                 │
              ▼                                 │
┌─────────────────────────────────────────────┴──────────────────────┐
│                            silicera (core)                          │
│  discovery · topology · fingerprint · knowledge · measurement       │
│  tournament · HNEP · SCF · remarks · archive · decision tree        │
└─────────────────────────────────────────────────────────────────────┘
```

## AMD exclusivity (real, not cosmetic)

Support requires AuthenticAMD + Zen3/4/5 knowledge-pack validation + topology suitable for fingerprinting. Unsupported CPUs exit gracefully (code 2).

## Artifacts

| Artifact | Role |
|----------|------|
| Knowledge pack | Public architecture facts |
| HNEP | Measured machine-bound profile |
| SCF | Compiler feedback JSON |
| Remarks YAML | Experimental LLVM-style remarks |
| Eval pack | Single-Zen campaign report |

## Non-goals

Kernel drivers, firmware/BIOS modification, overclocking, LLM-in-the-loop optimization, fabricated speedups, automatic cloud upload, multi-machine claims without a second measured host.

See `docs/INDEX.md` for the full documentation map.
