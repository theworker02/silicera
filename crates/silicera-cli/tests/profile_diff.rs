use silicera::{EnvironmentSnapshot, HardwareBackend, HnepProfile, KnowledgePack, MockHardware};
use std::path::PathBuf;
use std::process::Command;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "silicera-diff-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for file in ["before.hnep", "after.hnep"] {
            let _ = std::fs::remove_file(self.0.join(file));
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[test]
fn cli_diff_json_change_gate_and_invalid_files() {
    let dir = Fixture::new();
    let host = MockHardware::zen5_dual_ccd()
        .discover(&KnowledgePack::builtin())
        .unwrap();
    let mut profile = HnepProfile::from_tournaments(
        host.fingerprint.as_ref().unwrap(),
        EnvironmentSnapshot::capture(),
        &[],
        vec![],
        None,
        "before",
    )
    .unwrap();
    let before = dir.0.join("before.hnep");
    let after = dir.0.join("after.hnep");
    profile.write_to(&before).unwrap();
    profile.write_to(&after).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_silicera"))
            .args(["hnep", "diff"])
            .arg(&before)
            .arg(&after)
            .args(["--json", "--fail-on-change"])
            .output()
            .unwrap()
    };
    let output = run();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["changes"].as_array().unwrap().len(), 0);
    profile.header.label = "after".into();
    profile.recompute_digest().unwrap();
    profile.write_to(&after).unwrap();
    let output = run();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["changes"][0]["path"], "/header/label");
    std::fs::write(&after, "not JSON").unwrap();
    let output = run();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    std::fs::remove_file(&after).unwrap();
    assert!(!run().status.success());
}
