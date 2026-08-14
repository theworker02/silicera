//! Lightweight declarative variant helpers (macro experiment).
//!
//! Prefer typed registration over stringly hot-path lookup. This is intentionally
//! small — not a full proc-macro attribute system.

/// Declare a named specialization target and its baseline id for docs/codegen hints.
///
/// ```ignore
/// silicera::specialize_target! {
///     name: "memscan",
///     baseline: "scan_stride",
///     candidates: ["scan_dense", "copy", "copy_unrolled8"],
/// }
/// ```
#[macro_export]
macro_rules! specialize_target {
    (
        name: $name:expr,
        baseline: $baseline:expr,
        candidates: [$($cand:expr),* $(,)?] $(,)?
    ) => {{
        $crate::variant::VariantSetMeta {
            name: $name,
            baseline: $baseline,
            candidates: &[$($cand),*],
        }
    }};
}
