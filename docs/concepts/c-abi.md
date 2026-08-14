//! C ABI (experimental)
//!
//! Feature flag: `silicera-runtime` / `c-abi`.
//!
//! ```text
//! cargo build -p silicera-runtime --features c-abi
//! ```
//!
//! ## Symbols
//!
//! | Symbol | Purpose |
//! |--------|---------|
//! | `silicera_init` | Idempotent library init |
//! | `silicera_profile_load` | Load `.hnep` → opaque handle |
//! | `silicera_variant_select` | `size_bytes` → variant name into caller buffer |
//! | `silicera_profile_free` | Free handle |
//!
//! All `unsafe` lives in `crates/silicera-runtime/src/ffi.rs`. Prefer the safe
//! Rust `Dispatcher` API from Rust code.
//!
//! Status codes: `0` ok; `-1` null arg; `-2` I/O/parse; `-3` buffer too small.
//!
//! Not an AMD ABI. Not stable across Phase I minors without notice.
