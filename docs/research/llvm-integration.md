# LLVM integration (research)

Phase I does **not** modify LLVM. This document sketches a Phase II path for compiler engineers.

## Goals

- Emit or label specialization variants from LLVM IR / MachineIR remarks
- Allow Silicera tournaments to choose among compiler-produced clones
- Feed HNEP winners back as **orderings / preferences**, not as opaque ML suggestions

## Non-goals

- Replacing PGO or BOLT
- Requiring out-of-tree LLVM forks for Phase I demos
- Claiming official LLVM project status

## Strawman flow

```
clang/llvm -emit variants / remarks
        │
        ▼
silicera train (measure variants on host)
        │
        ▼
HNEP winners
        │
        ▼
runtime / link-time selection metadata
```

## Feedback format

See [compiler-feedback-format.md](compiler-feedback-format.md) (**SCF v1**, produced by `silicera feedback`).

## Dual-artifact codegen track

See [../benchmarks/native-artifacts.md](../benchmarks/native-artifacts.md) and `silicera measure native-artifacts`.

## Experimental YAML remarks

```bash
silicera export remarks out/profile.hnep -o out/remarks.yaml
silicera remarks-summary out/remarks.yaml
```

Produces a multi-document YAML stream (`silicera-llvm-remarks/1`) with `Pass`, `Function`, and `Args`
suitable for human review and as a bridge toward LLVM remark consumers. This is **not** the binary
bitstream remark format — see caveats in the file header.

## PGO / BOLT

See [../benchmarks/pgo-bolt.md](../benchmarks/pgo-bolt.md). Silicera does not bundle those tools.
