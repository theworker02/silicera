//! Property tests for fingerprint determinism.

use proptest::prelude::*;
use silicera::fingerprint::Fingerprint;
use silicera::knowledge::Microarch;
use silicera::topology::{ComputeDomain, CoreNode, Package, ThreadNode, TopologyGraph};

proptest! {
    #[test]
    fn fingerprint_rejects_non_slc(s in "[A-Za-z0-9]{0,40}") {
        prop_assume!(!s.starts_with("SLC:"));
        prop_assert!(Fingerprint::parse(&s).is_err());
    }

    #[test]
    fn topo_hash_deterministic(cores in 1u32..8) {
        let mut g = TopologyGraph::new();
        let mut pkg = Package { id: 0, domains: vec![] };
        let mut dom = ComputeDomain {
            id: 0,
            cores: vec![],
            shared_caches: vec![],
        };
        for i in 0..cores {
            dom.cores.push(CoreNode {
                id: i,
                threads: vec![ThreadNode { id: 0, apic_id: i }],
                l1i_bytes: 32768,
                l1d_bytes: 49152,
                l2_bytes: 1048576,
            });
        }
        pkg.domains.push(dom);
        g.packages.push(pkg);
        let a = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &g);
        let b = Fingerprint::from_topology(Microarch::Zen5, 0x1A, 0x44, 0, &g);
        prop_assert_eq!(a, b);
    }
}
