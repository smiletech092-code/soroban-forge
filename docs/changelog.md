# Generating the CHANGELOG

Soroban Forge uses [git-cliff](https://git-cliff.org) to produce a
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/)-compliant
`CHANGELOG.md` automatically from
[Conventional Commits](https://www.conventionalcommits.org).

## Prerequisites

Install **git-cliff** (requires Rust / Cargo):

```sh
cargo install git-cliff
```

Verify the installation:

```sh
git cliff --version
```

## Commit message format

Commits must follow the Conventional Commits spec so that git-cliff can
classify them:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

| Prefix | Changelog section |
|--------|-------------------|
| `feat` / `add` | Added |
| `fix` / `bug` | Fixed |
| `perf` | Performance |
| `refactor` / `change` | Changed |
| `chore` / `build` | Maintenance |
| `ci` | CI |
| `docs` | Docs |
| `test` | Testing |
| `revert` | Reverted |
| `remove` | Removed |
| `deprecate` | Deprecated |

Breaking changes must include `!` after the type or a `BREAKING CHANGE:` footer:

```
feat(scaffold)!: rename --template flag to --preset
```

## Commands

### Preview unreleased changes (dry run — nothing is written)

```sh
git cliff --unreleased
```

### Regenerate the full CHANGELOG from all tags

```sh
git cliff -o CHANGELOG.md
```

### Prepend only the unreleased section

```sh
git cliff --unreleased --prepend CHANGELOG.md
```

### Generate for a specific release tag

```sh
git cliff --tag v0.2.0 -o CHANGELOG.md
```

### Output to stdout (useful for release notes in CI)

```sh
git cliff --unreleased --strip all
```

## CI integration

A GitHub Actions step that publishes release notes automatically:

```yaml
- name: Generate release notes
  run: git cliff --unreleased --strip all > release_notes.txt

- name: Create GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    body_path: release_notes.txt
```

## Configuration

All git-cliff settings live in [`cliff.toml`](../cliff.toml) at the
repository root. The template, commit parsers, and tag pattern can all be
adjusted there without changing any scripts.
