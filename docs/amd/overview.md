# AMD support overview

Silicera supports **AMD Zen3, Zen4, and Zen5** microarchitectures when:

1. CPUID vendor is `AuthenticAMD`
2. Family/model maps through a knowledge pack
3. Topology validation does not hard-fail

There is **no** AMD partnership claim. Knowledge packs use public documentation and CPUID-visible geometry.

| Microarch | Typical family | Pack |
|-----------|----------------|------|
| Zen 3 | `19h` (selected models) | `profiles/amd/zen3.toml` |
| Zen 4 | `19h` (selected models) | `profiles/amd/zen4.toml` |
| Zen 5 | `1Ah` | `profiles/amd/zen5.toml` |

Unsupported AMD generations (e.g. Zen 2) and non-AMD vendors exit gracefully.
