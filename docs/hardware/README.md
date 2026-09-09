# Silicera hardware prototype program

Status: **proposed engineering brief, not fabrication-ready hardware**. Revision 0.1,
2026-09-09. No prototype has been built or qualified by this update. No budget,
board choice, production quantity, or manufacturer commitment is assumed.

## Product objective

Build a local, account-free AMD specialization appliance: an AMD Zen computer that
discovers its machine class, measures several correct implementations, records an
HNEP profile, and runs an application using the measured variant decisions.
The deliverable is a reproducible computer-plus-software system. Silicera currently
does not synthesize circuits, program an FPGA, accelerate arbitrary binaries, or
replace a CPU. Applications must supply and integrate their own variant handlers.

## Prototype alternatives

| Design | What is built | Evidence gained | Gate |
|---|---|---|---|
| P0: existing-machine demonstrator | Existing supported Zen3/4/5 host, stock firmware/cooling, Silicera and a variant-aware application | End-to-end measurement, profile reload, guarded dispatch, repeatability | Recommended first; no custom PCB |
| P1: instrumented appliance | Qualified commodity AMD board in serviceable enclosure; optional USB sensor controller | Thermal stability, power observations, manufacturing repeatability | P0 acceptance plus agreed budget |
| P2: productized system | Manufacturer-qualified AMD module/board, custom enclosure and optional carrier/instrumentation PCB | Supply continuity, assembly test, pilot production | P1 evidence and approved engineering quote |

Use the current machine before purchasing hardware. If a second host becomes
available, use it to test cross-machine portability and the existing Silicon Split
protocol; one machine cannot prove cross-SKU strategy divergence.

## Architecture and interfaces

```text
Application variants + baseline
        | correctness-checked tournaments
AMD host: discovery -> measurements -> HNEP -> guarded runtime -> application
        | run ID / timestamps                         | variant + reason
        +---------------- local evidence directory --+
Optional USB sensor controller -> timestamped telemetry sidecar
```

The host executes all specialized work. The controller, if built, only records
external measurements and displays status. Keep it outside the dispatch path;
disconnecting it must not invalidate a correct baseline implementation.

| Interface | Contract | Implementation status |
|---|---|---|
| Host discovery | Public CPUID, built-in AMD knowledge packs, environment snapshot | Implemented; exact board/CPU must be checked |
| Persistent profile | HNEP schema 1/2 loader, digest, fingerprint and variant IDs | Implemented; digest is corruption detection, not authentication |
| Runtime | `LoadedProfile::guarded_workload`, `guarded_size`, caller-provided compatible variant IDs | Implemented in 0.2; caller implements baseline and ISA checks |
| Revision review | `silicera hnep diff before.hnep after.hnep --json` | Implemented in 0.2 |
| Instrumentation | USB CDC serial, versioned newline-delimited records | Proposed; no USB driver, firmware, or importer implemented |
| Internal CPU power/counters | Optional separate AMD uProf evidence | No integrated counter reader; current Silicera reports unavailable |
| Security | OS permissions, local files; signed provisioning can be added later | TPM signing remains a stub |

## Host selection and software compatibility

Select an actual CPU/board/BIOS combination only after running `silicera inspect`,
reviewing knowledge-pack validation and checking cache/topology against vendor
documentation. A Zen marketing name alone is insufficient. Existing discovery
uses CPUID and some inferred topology; validate physical core/CCD layout before
using placement results as evidence. Do not assume GPU or NPU acceleration.

AMD offers an Embedded 9000 family using Zen 5; this makes it a possible later
evaluation route, **not a validated Silicera platform or an approved BOM**.
Availability, module ecosystem, BIOS support, cooling and procurement terms remain
vendor questions. [AMD Embedded 9000](https://www.amd.com/en/products/embedded/ryzen/9000-series.html).

Windows x86-64 is the first prototype target; Linux is a secondary validation
target. Record OS build, firmware, power policy, memory configuration, compiler,
Silicera version, fingerprint, and background activity with every campaign.
Use supported stable Rust; the inherited 1.75 manifest minimum is not a verified
minimum for the currently locked dependency graph.

## Mechanical and electrical concept

P0 retains the original host power supply and cooler. For P1, Minewing should
propose a serviceable enclosure around the selected board's actual drawings:
replaceable fan/filter, unobstructed intake/exhaust, accessible USB/service port,
strain relief, board mounting clearances, and no exposed conductive surfaces.
No dimensions can be released until the board, cooler and supply are selected.

Prefer a certified external supply or the motherboard vendor's validated PSU.
Size power delivery for the selected CPU's documented limits and transient load,
not average benchmark power. Keep mains circuitry off a first custom sensor PCB.
Do not tap CPU voltage rails, change VRM settings, or add host power-switching
circuitry to P0. Any later power switching requires graceful shutdown, recovery
testing, electrical review and an explicit requirement.

An optional sensor board can use a USB-capable microcontroller, inlet and exhaust
temperature sensors, and a status LED. Part numbers, voltage domains, connector
pinouts, ESD protection and PCB stack-up are manufacturer design deliverables.
External temperatures are not CPU junction temperature. An external power meter
measures system input, not isolated CPU energy. Calibrate and label both.

## Manufacturer quotation package

Ask Minewing to quote P0 assistance, P1 mechanical/instrumentation work and P2
custom electronics separately. Ask for one unit and a small pilot batch as separate
quantity tiers. Obtain legal entity, engineering contact, manufacturing location,
relevant AMD design experience, component sourcing strategy and subcontractor scope
before recording a partnership. This brief makes no claim about their location.

Required quote lines: engineering/NRE, board/module, memory, storage, cooling,
enclosure, supply, assembly, firmware, fixtures, calibration, compliance testing,
shipping, tooling ownership, lead times and substitutions. All costs are **TBD**;
there are no supplier quotes in this repository.

Before production approval require schematic review, editable CAD, Gerbers,
assembly drawings, BOM with approved alternates, firmware source/build procedure,
programming fixture specification, test logs, thermal report and failure analysis.
Agree ownership and licensing separately from the software's MIT/Apache licenses.
Market-specific EMC, electrical and environmental compliance must be assessed by
qualified engineers; this brief is not certification or permission to manufacture.

See [acceptance-test-plan.md](acceptance-test-plan.md) and
[instrumentation-contract.md](instrumentation-contract.md) for proposed test and
telemetry details.
