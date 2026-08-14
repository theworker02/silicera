//! Terminal styling helpers — restrained monochrome systems aesthetic.

#![allow(dead_code)]

/// Horizontal rule.
pub fn rule() -> String {
    "─".repeat(72)
}

/// Section header.
pub fn header(title: &str) -> String {
    format!("{}\n  {}\n{}", rule(), title, rule())
}

/// Key/value aligned row.
pub fn kv(key: &str, value: &str) -> String {
    format!("  {key:<18} {value}")
}

/// Status tag.
pub fn tag(ok: bool) -> &'static str {
    if ok {
        "OK"
    } else {
        "FAIL"
    }
}
