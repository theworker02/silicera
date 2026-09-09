# Prototype acceptance and evidence plan

These are proposed engineering gates. They have not been run on manufactured hardware.

## P0 bring-up

1. Record serial/asset ID locally, CPU SKU, board revision, firmware version, RAM
   population/speed, OS build, power policy, cooler and software commit. Redact serials
   before sharing; a Silicera fingerprint is a machine class, not a unit serial.
2. Run the following commands from the repository. Preserve stdout, stderr and exit
   status beside the artifacts; do not suppress failed checks.

```sh
cargo build --release --locked
cargo test --workspace --locked
silicera doctor
silicera inspect
silicera report -o out/host-report.md
silicera calm-check
silicera train -o out/before.hnep --label prototype-p0
silicera verify out/before.hnep --strict-machine --spot-check
silicera health out/before.hnep
silicera eval -o out/eval.json
silicera train -o out/after.hnep --label prototype-p0
silicera hnep diff out/before.hnep out/after.hnep --json
```

Use `target/release/silicera` (Windows: `target\release\silicera.exe`) in place of
`silicera` unless the built binary is already on PATH. Real host commands are not
mock benchmarks. Unsupported discovery is a failed platform qualification gate,
not a reason to override the fingerprint.

## Application demonstration

Create a real embedding application with a scalar baseline and at least one
correctness-tested specialized implementation for one useful workload (for example
buffer scanning). Register only variants whose required CPU features are available.
Choose a measured workload entry from the application's own training; do not map
an unrelated benchmark winner onto an application function just because its ID matches.

Load the HNEP, call `guarded_workload` with a Medium or High confidence floor, map
the returned variant ID to a function and execute it. Record the decision reason.
The library selector exists; this manufacturer-specific embedding application is
still a deliverable, not a shipped demonstration device.

## Acceptance matrix

| Gate | Procedure | Pass condition |
|---|---|---|
| Discovery | Compare inspect data with exact CPU/board documentation | Supported host, discrepancies resolved |
| Correctness | Compare baseline and candidates on empty, boundary, random and large inputs | Identical expected results under documented numerical tolerances |
| Persistence | Save profile, restart application, reload | Digest verifies and decisions match for unchanged profile/host |
| Tamper detection | Modify a saved winner without recomputing digest | Load fails; no specialized execution |
| Wrong host | Load on a genuinely different host; use mocks only for software CI | Fallback policy selects baseline; strict policy returns an error |
| Missing implementation | Omit winning ID from runtime registry | `unavailable_variant` and baseline |
| Weak evidence | Set confidence floor above measured confidence | `insufficient_confidence` and baseline |
| Size boundary | Probe threshold-1, threshold and threshold+1 | Expected less-than / greater-or-equal branch; registry guard holds |
| Repeatability | At least 3 sessions after separate boots, at least 20 iterations per run | Report medians/spread/winner changes; inconclusive remains inconclusive |
| Sustained workload | Proposed 60-minute run at agreed ambient/load | No crash, corruption or thermal limit violation; log throttling honestly |
| Controller loss (P1) | Unplug optional controller | Host continues; telemetry gap marked, never filled with invented data |
| Power loss recovery (P1) | Controlled test using vendor-approved procedure | Filesystem/application recover; partial artifacts rejected |

Performance success is workload-specific and is not required to be a speedup.
A baseline win is valid evidence. Set any commercial improvement requirement only
after P0 data exists. Repeat sessions with controlled power policy, consistent
ambient conditions and minimal background load; record all deviations.

## Optional power and thermal study

Use a calibrated external meter or separately installed AMD uProf where supported.
AMD documents system power profiling in its [uProf guide](https://docs.amd.com/r/en-US/57368-uProf-user-guide/System-wide-Power-Profiling-Live?contentId=U4ssFIIF50yuKr6wG3NoSw).
Verify event availability and required privileges on the chosen host. Silicera does
not install this tool or collect its counters. Keep these exports as separate evidence.

Report power measurement point, units, sample rate, calibration, time alignment and
missing samples. Compare energy only over equivalent, correctness-checked work and
equivalent system boundaries. Never interpret a sensor-board reading as CPU-only power.

## Exit criteria

P0: reproducible build, supported host, correct application integration, tested
fallbacks and repeatability report. P1: P0 plus reviewed assembly, thermal evidence,
calibrated instrumentation and recoverability. P2: P1 plus signed requirements,
manufacturer design review, supply/alternates plan and agreed compliance test plan.
No stage authorizes purchasing, ordering fabrication or shipping units automatically.
