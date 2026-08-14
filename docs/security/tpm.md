# TPM binding (experimental stub)

This release includes `silicera::tpm::TpmBindingStub`:

- Reports `Stubbed` availability on typical developer hosts
- Does **not** participate in specialization or dispatch
- `bind_digest` returns a clear “not implemented” error

## Intent

Explore hardware-bound profile signatures where a TPM is available, without blocking the product on Windows TPM API complexity.

## Separation

Specialization correctness must never depend on TPM presence. Profiles remain usable without attestation.
