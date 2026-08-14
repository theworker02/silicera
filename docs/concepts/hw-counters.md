//! Hardware counters (optional)
//!
//! Phase I specialization uses wall-clock timers (`MeasurementEngine`).
//!
//! `silicera doctor` probes counters via `probe_counters()`:
//!
//! - **Windows:** unprivileged PMC access is **UNAVAILABLE** (would need elevated
//!   ETW/PMC or a driver). Silicera does not request privileges.
//! - **Linux:** Phase I does not open `perf_event_open`; reported UNAVAILABLE
//!   until a least-privilege path is validated.
//!
//! Specialization and harnesses continue without counters. Do not invent PMC
//! values in published tables.
