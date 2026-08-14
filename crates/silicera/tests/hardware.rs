//! Real AMD hardware tests — ignored in CI.
//!
//! Run on a supported Zen host:
//! ```bash
//! cargo test -p silicera --test hardware -- --ignored --nocapture
//! ```

use silicera::hardware::{detect_hardware, require_supported};

#[test]
#[ignore = "requires real AMD Zen3/Zen4/Zen5 host; see docs/amd/support.md"]
fn real_host_is_supported_amd() {
    let info = detect_hardware().expect("discovery should not panic");
    let micro = require_supported(&info).expect("host should be supported AMD Zen");
    eprintln!("brand: {}", info.brand);
    eprintln!("microarch: {micro}");
    eprintln!(
        "fingerprint: {}",
        info.fingerprint.as_ref().unwrap().value
    );
    assert!(!info.is_mock);
    assert!(info.topology.core_count() > 0);
}
