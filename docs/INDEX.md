# Silicera Documentation Index

Professional entry point for AMD engineers, LLVM developers, and systems researchers.

## Start here

| Doc | Audience |
|-----|----------|
| [../README.md](../README.md) | Project overview |
| [../ARCHITECTURE.md](../ARCHITECTURE.md) | System design |
| [../ROADMAP.md](../ROADMAP.md) | Phases (single-Zen focus) |
| [getting-started/index.md](getting-started/index.md) | First measurements |

## Concepts

- [specialization.md](concepts/specialization.md)
- [variants.md](concepts/variants.md)
- [measurements.md](concepts/measurements.md)
- [hnep.md](concepts/hnep.md)
- [hardware-identity.md](concepts/hardware-identity.md)
- [c-abi.md](concepts/c-abi.md)
- [hw-counters.md](concepts/hw-counters.md)

## AMD

- [overview.md](amd/overview.md)
- [topology.md](amd/topology.md)
- [support.md](amd/support.md)

## Benchmarks (single-Zen)

- [single-machine.md](benchmarks/single-machine.md) — primary evidence path
- [native-artifacts.md](benchmarks/native-artifacts.md)
- [portable-vs-native.md](benchmarks/portable-vs-native.md)
- [size-classes.md](benchmarks/size-classes.md)
- [alignment.md](benchmarks/alignment.md)
- [placement.md](benchmarks/placement.md)
- [pgo-bolt.md](benchmarks/pgo-bolt.md)

## Research

- [silicera-paper.md](research/silicera-paper.md)
- [hnep-spec.md](research/hnep-spec.md)
- [compiler-feedback-format.md](research/compiler-feedback-format.md)
- [llvm-integration.md](research/llvm-integration.md)
- [silicon-split.md](research/silicon-split.md) — deferred without Machine B
- [amd-integration.md](research/amd-integration.md)
- [hne-concept.md](research/hne-concept.md)

## Security

- [fingerprint.md](security/fingerprint.md)
- [tpm.md](security/tpm.md)

## Site

Static research site under [`../site/`](../site/) — review before deploying.

## Profile revisions and hardware prototypes

- [Profile revision workflow](concepts/profile-revisions.md)
- [Hardware prototype program](hardware/README.md)
- [Prototype acceptance tests](hardware/acceptance-test-plan.md)
- [Proposed instrumentation contract](hardware/instrumentation-contract.md)
