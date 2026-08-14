#![no_main]
use libfuzzer_sys::fuzz_target;
use silicera::hnep::HnepProfile;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = HnepProfile::parse_str(s);
    }
});
