# Security Policy

## Supported versions

The `0.1.x` line is research software. Security fixes are applied on a best-effort basis to the latest `main` / release tag.

## Scope

Silicera runs in **user mode** only:

- No kernel driver
- No intentional BIOS/firmware modification
- No MSR write paths in this release
- Fingerprints are **not** authentication credentials (topology/cache geometry only)

Optional TPM binding is experimental and stubbed; it does not gate specialization. See `docs/security/tpm.md` and `docs/security/fingerprint.md`.

## Reporting

Report vulnerabilities privately to the maintainers (private security advisory on the hosting forge when available, or the contact listed in repository metadata).

Include:

- Silicera version / git revision
- Host OS and CPU (vendor/family/model)
- Reproduction steps
- Impact assessment

## Non-security issues

Incorrect speedup claims, documentation errors, and unsupported-CPU handling belong in ordinary issue trackers — thank you for keeping the evidence culture honest.
