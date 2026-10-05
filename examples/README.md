# Examples

Checked-in output of `soroban-forge`, kept for browsing without installing the
tool. Owned by Module 5 (docs & DX).

## Regenerating examples

CI (`.github/workflows/examples-drift.yml`) regenerates every template example
below on every push and PR and fails if the checked-in tree differs from what
the template produces today. `.github/workflows/scaffold-templates.yml` then
builds and tests each checked-in example, including its contract WASM:

```sh
cargo build --release --bin soroban-forge
scripts/regenerate-examples.py --check
```

To update stale examples after changing templates, run the same script without
`--check` (it discovers every template directory and overwrites the matching
example):

```sh
cargo build --release --bin soroban-forge
scripts/regenerate-examples.py
```

The check ignores `Cargo.lock`, `README.md`, and `target/`: lockfiles are
resolved against live crates.io state, and example READMEs can include
additional project-specific guidance. Generated manifests, source, and
configuration files are still compared with the templates.

**Known gap:** every template pins `rust-toolchain.toml` to `1.84`, but some
transitive dependencies (including `soroban-sdk` itself, as of writing) have
since moved to editions and MSRVs newer than that pin. A fresh
`cargo generate-lockfile` in any example may resolve versions that later fail
`cargo test` under the pinned toolchain with an "edition2024 is required"
error — this reproduces on `hello-forge` too if you regenerate it from
scratch, so it isn't specific to any one example. That's a toolchain/ecosystem
mismatch across all templates, tracked separately from example drift.

`hello-forge` is an additional, manually generated example that demonstrates
`test-init` and `ci-init`; the `hello-world` directory is the checked-in
example corresponding directly to that template:

```sh
# From the repo root:
cd examples
soroban-forge new hello-forge --template hello-world
soroban-forge test-init --force
soroban-forge ci-init --deploy
```

These directories are excluded from the cargo workspace (they are standalone
projects).

Each bundled template has a same-named scaffolded project under `examples/`.
`hello-forge/` is an additional hello-world example with a generated test
harness and CI workflows. See the
[template catalogue](../docs/template-catalogue.md) for descriptions.
