# Comparing Silicera to PGO and BOLT

**Status:** Phase II research methodology (no mandatory toolchain dependency)

Silicera does not replace LLVM PGO or BOLT. This document defines how to compare
them honestly on shared workloads.

## What each tool optimizes

| Tool | Input | Output |
|------|-------|--------|
| LLVM PGO | Execution counts / branch weights | Reordered blocks, inlining, etc. |
| BOLT | Sampled binary profiles | Layout / icache-aware reordering |
| Silicera | Machine fingerprint + measured variants | HNEP winners + decision tree |

## Fair comparison protocol

1. **Same source** and same portable baseline binary.
2. **Same inputs** and iteration counts for measurement.
3. Arms:
   - `portable` — generic x86-64 build
   - `pgo` — instrument → train → optimize (document clang/rustc flags used)
   - `bolt` — optional post-link (document version)
   - `silicera` — HNEP dispatch after `silicera train`
   - `native_artifact` — `-C target-cpu=native` (see `silicera native-artifacts`)
4. Report **raw medians**, confidence, and losses. If PGO or BOLT beat Silicera, print it.
5. Export a [`silicera repro-export`](../concepts/) pack with flags and fingerprint.

## What Silicera uniquely claims

Machine-bound variant selection and size-class decision trees that **fail closed**
to baseline on fingerprint mismatch — not a substitute for profile-guided layout.

## Non-goals

- Bundling BOLT/PGO binaries in the Silicera repo
- Claiming superiority without measured tables
- Requiring AMD engineers to install a Silicera cloud service

## Related

- [portable-vs-native.md](portable-vs-native.md)
- [compiler-feedback-format.md](../research/compiler-feedback-format.md)
- [llvm-integration.md](../research/llvm-integration.md)
