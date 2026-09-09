# Proposed instrumentation contract v0

**Design only.** No firmware, serial transport, importer, schematic or production
protocol implementation is included. Review with the selected controller vendor.

USB CDC serial is the proposed first transport, using newline-delimited UTF-8 JSON.
Each record is at most 4096 bytes including newline. Unknown protocol versions,
oversized records, invalid JSON, duplicate sequence numbers and invalid units must
be rejected by a future importer; never execute record contents as commands.
Discovery must be an explicit local user action, not automatic USB trust.

Illustrative sample (synthetic, not a measurement):

```json
{"protocol":"silicera-instrumentation/0","kind":"sample","run_id":"example-only","device_id":"fixture-a","sequence":42,"device_uptime_ms":12000,"sensor":"inlet_temperature","value":24.5,"unit":"degC","quality":"synthetic"}
```

Required samples: protocol, kind, run_id, device_id, sequence (unsigned 64-bit),
device_uptime_ms (unsigned 64-bit), sensor, finite numeric value, unit, quality.
Real quality values are `valid`, `uncalibrated`, `fault`; `synthetic` is restricted
to test fixtures. Sensor allowlist initially: `inlet_temperature` and
`exhaust_temperature`, both `degC`. Power is deferred until the measurement circuit
and calibration are reviewed. Fault records use `value: null`, never zero.

A future host recorder adds host monotonic receive time and the profile digest to
its sidecar. Controller reset starts a new session; sequence and uptime restarting
must never be silently concatenated. Record gaps, disconnects and duplicate records.
Pair run ID, device session and timestamps explicitly; independent device and host
clocks are not synchronized merely because timestamps look similar.

Suggested P1 acquisition target: 1 sample/second per temperature channel, evaluated
for adequacy during thermal testing. This cannot resolve microsecond benchmark events.
Record sensor model, firmware revision, calibration date, reference instrument and
uncertainty. Set operational thresholds from the selected parts' datasheets.

Proposed host messages: `hello` (version negotiation), `start` (bounded run ID),
`stop`. A future implementation must validate them and return acknowledged state
or structured errors. No arbitrary shell, firmware update, fan control or power
switching command is part of v0. Disconnect or malformed input returns acquisition
to idle; host workload execution remains independent.

Keep raw telemetry out of HNEP v2. It is an optional, bounded sidecar supporting a
measurement report, not authority to change a winner. Never publish hardware serials
or send telemetry to a cloud service by default.
