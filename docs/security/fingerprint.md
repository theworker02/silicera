# Fingerprints and security

Silicera machine fingerprints are **identifiers for profile matching**, not authenticators.

## Guarantees (this release)

- No serial numbers or disk identifiers in the fingerprint string
- HNEP integrity digest detects accidental corruption / casual tampering
- Integrity digest is **not** a hardware root of trust

## Non-guarantees

- Fingerprints are not secret
- An attacker who can write profiles can supply a malicious HNEP; treat profiles like other build artifacts
- TPM binding is optional and stubbed — see [tpm.md](tpm.md)

Operators should distribute HNEP files through the same trust channels as binaries.
