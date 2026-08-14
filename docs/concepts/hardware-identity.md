# Hardware identity

Silicera fingerprints identify a **class of machine configuration** suitable for matching HNEP files.

```
SLC:AMD:ZENn:<family>:<model>:<stepping>:<topo_hash>:<cache_hash>
```

## Properties

- Derived from CPUID + topology/cache geometry hashes
- Does **not** include serial numbers, MAC addresses, or disk IDs
- **Not** an authentication secret
- Collision resistance is for profile hygiene, not security proofs

## Matching

| Match | Runtime behavior (default) |
|-------|----------------------------|
| Exact fingerprint | Use profile |
| Same silicon class, different topo/cache hash | `PROFILE MISMATCH` → baseline |
| Different class / unsupported | `PROFILE MISMATCH` → baseline |
| `--strict-machine` | Error instead of specialized dispatch |

See [../security/fingerprint.md](../security/fingerprint.md).
