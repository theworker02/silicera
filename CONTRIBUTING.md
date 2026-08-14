# Contributing

Thank you for interest in Silicera. This is experimental research software aimed at AMD Zen systems researchers and compiler engineers.

## Ground rules

1. **No fabricated benchmarks.** Numbers in docs or `site/` must come from recorded `silicera` runs with environment snapshots.
2. **No AMD affiliation claims.** Do not imply partnership, certification, or endorsement.
3. **Correctness before speed.** Tournaments keep correctness gates and regression rejection.
4. **Four crates.** Do not add a fifth primary crate without an architecture discussion.
5. **License.** Contributions are dual-licensed MIT OR Apache-2.0.

## Development

```bash
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
```

Real AMD hardware tests:

```bash
cargo test --workspace -- --ignored
```

Lab demos (measured on your host):

```bash
cargo run -p silicera-lab --example silicon_split
cargo run -p silicera-lab --example wrong_machine
```

See [examples/README.md](examples/README.md). Optional fuzzing: [fuzz/README.md](fuzz/README.md).

## Documentation tone

Calm, precise, systems-oriented. Prefer empty benchmark tables over invented speedups. When extending knowledge packs, document how to validate — do not expand the SKU matrix with unverified claims.

## Pull requests

- Prefer small, focused changes
- Update `CHANGELOG.md` for user-visible behavior
- Add `SAFETY` comments for every `unsafe` block

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
