# soroban-forge Homebrew Tap

This repository is the official [Homebrew](https://brew.sh) tap for
[soroban-forge](https://github.com/soroban-forge-labs/soroban-forge).

It is intended to be published as a **separate GitHub repository** named
`soroban-forge-labs/homebrew-tap`.  The contents of this directory map
directly to the root of that repository — copy or symlink them when setting
the tap repo up.

## Install

```sh
brew tap soroban-forge-labs/tap
brew install soroban-forge
```

Or in a single command:

```sh
brew install soroban-forge-labs/tap/soroban-forge
```

## Upgrade

```sh
brew upgrade soroban-forge
```

## Uninstall

```sh
brew uninstall soroban-forge
brew untap soroban-forge-labs/tap   # optional: remove the tap itself
```

## How it works

The formula in [`Formula/soroban-forge.rb`](Formula/soroban-forge.rb) downloads
a **pre-compiled binary** from the GitHub Releases page — no Rust toolchain is
required on the user's machine.

Supported platforms:

| Platform          | Architecture | Archive                                              |
|-------------------|--------------|------------------------------------------------------|
| macOS             | Apple Silicon (`arm64`) | `soroban-forge-<version>-aarch64-apple-darwin.tar.gz` |
| macOS             | Intel (`x86_64`) | `soroban-forge-<version>-x86_64-apple-darwin.tar.gz`  |
| Linux             | `arm64`       | `soroban-forge-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| Linux             | `x86_64`      | `soroban-forge-<version>-x86_64-unknown-linux-gnu.tar.gz`  |

Each archive contains a single `soroban-forge` binary at its root, so `install`
is just `bin.install`. On top of that the formula generates shell completions
for bash, zsh, and fish, and installs man pages when the binary provides them
-- users do not need to wire either up by hand.

## Releasing a new version

Releases are automated. Pushing a `v*` tag to the main repository runs its
[`release` workflow](https://github.com/soroban-forge-labs/soroban-forge/blob/main/.github/workflows/release.yml),
which:

1. Checks the tag against `[workspace.package].version` in `Cargo.toml` and
   stops if they disagree.
2. Cross-builds `soroban-forge` for every supported target and packages each
   binary as a flat archive.
3. Publishes the archives plus a `SHA256SUMS.txt` manifest to GitHub Releases.
4. Regenerates this formula with the real checksums and, when the
   `HOMEBREW_TAP_TOKEN` secret is configured, commits it to this repository.

```sh
# from the main repository
./scripts/release.sh 0.2.0
```

### Updating the formula by hand

`Formula/soroban-forge.rb` is generated -- do not edit it directly. The
generator lives in the main repository and is the single source of truth, so
the checked-in formula and the release tooling cannot drift apart:

```sh
# in a clone of soroban-forge-labs/soroban-forge
curl -fsSLO https://github.com/soroban-forge-labs/soroban-forge/releases/download/v0.2.0/SHA256SUMS.txt
./scripts/update-homebrew-formula.sh --version 0.2.0 --checksums SHA256SUMS.txt
```

Then copy `homebrew-tap/Formula/soroban-forge.rb` into this repository at
`Formula/soroban-forge.rb`. Every published release also attaches the rendered
formula as a `soroban-forge.rb` asset, which you can drop in as-is.
