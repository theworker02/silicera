# Reviewing and consuming profile revisions

Train to a new path so the previous profile remains available for comparison:

```sh
silicera train -o out/next.hnep
silicera hnep diff out/current.hnep out/next.hnep
silicera hnep diff out/current.hnep out/next.hnep --json --fail-on-change
```

The diff validates both profiles without detecting the current CPU. It is useful
on a non-AMD review machine. Workloads and size classes are matched by identity,
not array position. Duplicate identities are rejected. Header, environment, entry
and tree field changes are reported; `header.created_at` and `digest` are excluded.
Thus an identical retraining snapshot with only a new timestamp has no changes.
Labels and producer versions are meaningful changes. Paths use JSON pointer
escaping (`~0` and `~1`) with entry names replacing indexes.

The versioned JSON report contains `format`, `same_fingerprint`, and `changes`.
Each change has `kind` (`added`, `removed`, `modified`), `path`, `before` and `after`;
missing sides serialize as null. Use `kind` to distinguish absence from a JSON null.
`--fail-on-change`
returns exit 1 after printing the report when fields differ. Invalid/unreadable
profiles also fail, so automation must inspect stderr and parse stdout only when
it contains a complete report. No changes returns 0.

Timing changes do not establish a speedup or regression. Review run conditions,
correctness and confidence before deploying a profile. Different fingerprints
are reported explicitly; the diff never rewrites either artifact.

## Guarding an embedding application's dispatch

```rust,ignore
use silicera::hnep::Confidence;
use silicera_runtime::{Dispatcher, MismatchPolicy};

let dispatcher = Dispatcher::open(
    std::path::Path::new("out/current.hnep"),
    MismatchPolicy::FallbackBaseline,
)?;
// Include only implemented variants compatible with this host's ISA.
let available = ["baseline", "my_portable_scan"];
let decision = dispatcher.loaded().guarded_workload(
    "my_scan", &available, Confidence::Medium,
);
// Map decision.variant to an actual function in your application.
// The application must always implement baseline, even for an empty registry.
println!("{}: {:?}", decision.variant, decision.reason);
```

The selectors return borrowed strings without heap allocation. Workload dispatch
checks machine matching, presence, explicit baseline, confidence, and availability
in that order. Size dispatch checks machine matching, tree presence, explicit
baseline and availability. Tree leaves have no confidence metadata, so guarded
size selection does not apply a confidence floor. Neither method executes code or
detects the CPU features required by an arbitrary application-defined variant ID.

Reasons are serialized as `selected`, `profile_baseline`, `machine_mismatch`,
`missing_workload`, `missing_tree`, `insufficient_confidence`, or
`unavailable_variant`. Existing `workload`/`size` APIs retain their behavior.

Direct `LoadedProfile::from_parts` construction now verifies schema and digest
with the same parser used for files. Unsupported hosts fail under `StrictMachine`;
fallback policy still selects baseline. Digest verification is not authentication.
