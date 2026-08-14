# Roadmap

Silicera is independent systems research software. This document is a plan, not a promise of vendor engagement.

## Operating constraint

**Primary evidence path = one physical Zen host** (developer SKU).  
Silicon Split Machine B / multi-SKU YES–NO claims are **deferred** until a second Zen machine is available. Single-machine work is not blocked.

## Current release (absorbs former Phase I–II)

Shipped for Zen3 / Zen4 / Zen5 on the developer SKU:

- [x] Four-crate workspace, discovery, HNEP v2, tournaments, runtime fallback, CLI / lab
- [x] Silicon Split *protocol* + placeholder (verdict UNKNOWN without Machine B)
- [x] SCF feedback, host-ISA arms, alignment / placement / arch-compare / spot-check / fleet-share
- [x] Native artifact track + suite + HNEP merge
- [x] Calm-check + `silicera eval` pack + repro sidecar
- [x] GitHub Actions CI (MockHardware)
- [x] Experimental LLVM-style YAML remarks + `remarks-summary`
- [x] Measured results archive, shell completions, brand / `RELEASE_LINE`
- [x] Profile lifecycle health (`silicera health`) and host Markdown `report`
- [x] Toolchain presence probe for PGO/BOLT readiness docs
- [ ] Optional cargo-fuzz in CI
- [ ] Optional PGO/BOLT tables when those toolchains are installed locally
- [ ] Deeper LLVM remark / multiversioning consumer prototype

## Next (when a second Zen host exists)

- Silicon Split with real Machine B (measured YES/NO — never fabricated)
- Cross-SKU measured strategy divergence tables

## Hardening

- Stronger staleness / optional TPM signing (beyond the current stub)
- Stronger OS topology APIs

## Explicit non-goals

- Fabricated percentages; AMD partnership claims; kernel/firmware mods; LLM optimization loops; automatic cloud upload
- Claiming multi-machine divergence without a second measured host
